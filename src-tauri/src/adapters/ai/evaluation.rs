use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use serde::Deserialize;

use crate::{
    adapters::{
        ai::{cloud_asr::CloudSpeechRecognizer, cloud_rewriter::CloudTextRewriter},
        audio::cpal_capture::resample_mono_pcm16,
        secrets::credential_store::{wipe_string, CredentialStore},
    },
    core::{
        models::{DictionaryEntry, RecordedAudio, RewriteMode},
        ports::{SpeechRecognizer, TextRewriter},
    },
    services::voice::{apply_dictionary, dictionary_terms_in_transcript, dictionary_vocabulary},
};

const TARGET_SAMPLE_RATE: u32 = 16_000;

#[derive(Deserialize)]
struct EvaluationManifest {
    language: String,
    cases: Vec<EvaluationCase>,
}

#[derive(Deserialize)]
struct EvaluationCase {
    id: String,
    category: String,
    audio: String,
    reference: String,
    #[serde(default = "default_max_cer")]
    max_asr_cer: f64,
    #[serde(default)]
    dictionary: Vec<EvaluationDictionaryEntry>,
    #[serde(default)]
    final_must_contain: Vec<String>,
    #[serde(default)]
    final_must_not_contain: Vec<String>,
}

#[derive(Deserialize)]
struct EvaluationDictionaryEntry {
    source: String,
    replacement: String,
}

fn default_max_cer() -> f64 {
    0.20
}

#[tokio::test]
#[ignore = "requires local WAV corpus and a configured Qwen credential; consumes model quota"]
async fn online_voice_pipeline_evaluation() {
    let manifest_path = std::env::var("LISTEN_TO_ME_EVAL_MANIFEST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../evaluation/manifest.local.json")
        });
    let manifest_path = manifest_path.as_path();
    let manifest: EvaluationManifest =
        serde_json::from_str(&fs::read_to_string(manifest_path).unwrap_or_else(|error| {
            panic!(
                "failed to read {}: {error}; copy evaluation/manifest.example.json first",
                manifest_path.display()
            )
        }))
        .expect("valid evaluation manifest");
    assert!(!manifest.cases.is_empty(), "evaluation corpus is empty");

    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let mut api_key = std::env::var("DASHSCOPE_API_KEY")
        .or_else(|_| CredentialStore::qwen_api_key())
        .expect("configure the Qwen credential in the app or DASHSCOPE_API_KEY");
    let rewriter = CloudTextRewriter::new(
        api_key.clone(),
        crate::core::models::DEFAULT_QWEN_REWRITE_MODEL,
    )
    .expect("valid Qwen credential");
    let mut recognizer =
        CloudSpeechRecognizer::new(api_key.clone()).expect("valid Qwen credential");
    wipe_string(&mut api_key);
    let mut total_distance = 0usize;
    let mut total_reference_chars = 0usize;
    let mut total_asr_ms = 0u128;
    let mut total_rewrite_ms = 0u128;
    let case_count = manifest.cases.len();
    let mut failures = Vec::new();

    for case in manifest.cases {
        let dictionary = case
            .dictionary
            .iter()
            .enumerate()
            .map(|(index, entry)| DictionaryEntry {
                id: index as i64,
                source: entry.source.clone(),
                replacement: entry.replacement.clone(),
                category: "评测".to_owned(),
                updated_at: 0,
            })
            .collect::<Vec<_>>();
        let vocabulary = dictionary_vocabulary(&dictionary);
        recognizer.configure(&manifest.language, vocabulary);

        let audio_path = manifest_dir.join(&case.audio);
        let audio = read_wav(&audio_path)
            .unwrap_or_else(|error| panic!("{} failed to load audio: {error}", case.id));
        let asr_started = Instant::now();
        let transcript = recognizer
            .transcribe(audio)
            .await
            .unwrap_or_else(|error| panic!("{} ASR request failed: {error}", case.id));
        let asr_ms = asr_started.elapsed().as_millis();
        total_asr_ms += asr_ms;
        let corrected = apply_dictionary(&transcript, &dictionary);
        let (distance, reference_chars) = character_error_counts(&case.reference, &corrected.0);
        let cer = distance as f64 / reference_chars.max(1) as f64;
        total_distance += distance;
        total_reference_chars += reference_chars;

        let protected_terms = dictionary_terms_in_transcript(&corrected, &dictionary);
        let rewrite_started = Instant::now();
        let final_text = rewriter
            .rewrite(corrected.clone(), RewriteMode::Clean, protected_terms)
            .await
            .unwrap_or_else(|error| panic!("{} rewrite request failed: {error}", case.id))
            .0;
        let rewrite_ms = rewrite_started.elapsed().as_millis();
        total_rewrite_ms += rewrite_ms;

        println!(
            "[{}:{}] CER={:.2}% ASR={}ms REWRITE={}ms\n  ASR: {}\n  FINAL: {}",
            case.category,
            case.id,
            cer * 100.0,
            asr_ms,
            rewrite_ms,
            corrected.0,
            final_text
        );
        if cer > case.max_asr_cer {
            failures.push(format!(
                "{} CER {:.2}% exceeds {:.2}%",
                case.id,
                cer * 100.0,
                case.max_asr_cer * 100.0
            ));
        }
        for expected in case.final_must_contain {
            if !final_text.contains(&expected) {
                failures.push(format!(
                    "{} final text must contain {expected:?}, got {final_text:?}",
                    case.id
                ));
            }
        }
        for forbidden in case.final_must_not_contain {
            if final_text.contains(&forbidden) {
                failures.push(format!(
                    "{} final text must not contain {forbidden:?}, got {final_text:?}",
                    case.id
                ));
            }
        }
    }

    println!(
        "[TOTAL] micro CER={:.2}% mean ASR={}ms mean REWRITE={}ms (distance={}, reference_chars={})",
        total_distance as f64 / total_reference_chars.max(1) as f64 * 100.0,
        total_asr_ms / case_count.max(1) as u128,
        total_rewrite_ms / case_count.max(1) as u128,
        total_distance,
        total_reference_chars
    );
    assert!(
        failures.is_empty(),
        "voice evaluation failures:\n{}",
        failures.join("\n")
    );
}

fn read_wav(path: &Path) -> Result<RecordedAudio, String> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let spec = reader.spec();
    if spec.channels == 0 || spec.sample_rate == 0 {
        return Err("WAV has an invalid channel count or sample rate".to_owned());
    }

    let interleaved = match spec.sample_format {
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to decode PCM16 WAV: {error}"))?,
        hound::SampleFormat::Int => {
            let shift = spec.bits_per_sample.saturating_sub(16);
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|value| (value >> shift) as i16))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("failed to decode integer WAV: {error}"))?
        }
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| {
                sample.map(|value| (value.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to decode float WAV: {error}"))?,
    };
    let channels = usize::from(spec.channels);
    let mono = interleaved
        .chunks(channels)
        .map(|frame| {
            let sum = frame.iter().map(|sample| i32::from(*sample)).sum::<i32>();
            (sum / frame.len() as i32) as i16
        })
        .collect::<Vec<_>>();

    Ok(RecordedAudio {
        samples: resample_mono_pcm16(&mono, spec.sample_rate, TARGET_SAMPLE_RATE),
        sample_rate: TARGET_SAMPLE_RATE,
    })
}

fn character_error_counts(reference: &str, hypothesis: &str) -> (usize, usize) {
    let reference = normalized_characters(reference);
    let hypothesis = normalized_characters(hypothesis);
    (
        levenshtein_distance(&reference, &hypothesis),
        reference.len(),
    )
}

fn normalized_characters(text: &str) -> Vec<char> {
    normalize_equivalent_forms(&text.to_lowercase())
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect()
}

fn normalize_equivalent_forms(text: &str) -> String {
    let mut normalized = text.replace(":00", "点").replace(":30", "点半");
    for (chinese, digit) in [
        ("零", "0"),
        ("一", "1"),
        ("二", "2"),
        ("三", "3"),
        ("四", "4"),
        ("五", "5"),
        ("六", "6"),
        ("七", "7"),
        ("八", "8"),
        ("九", "9"),
        ("十", "10"),
    ] {
        normalized = normalized.replace(&format!("{chinese}点"), &format!("{digit}点"));
    }
    normalized
}

fn levenshtein_distance(left: &[char], right: &[char]) -> usize {
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];
    for (left_index, left_character) in left.iter().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_character) in right.iter().enumerate() {
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + usize::from(left_character != right_character));
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cer_ignores_case_spaces_and_punctuation() {
        assert_eq!(character_error_counts("使用 Codex。", "使用codex"), (0, 7));
    }

    #[test]
    fn cer_counts_substitution_insertion_and_deletion() {
        assert_eq!(character_error_counts("今天开会", "明天开个会"), (2, 4));
    }

    #[test]
    fn cer_treats_equivalent_clock_formats_as_equal() {
        assert_eq!(
            character_error_counts("明天下午三点开会", "明天下午 3:00 开会"),
            (0, 8)
        );
        assert_eq!(character_error_counts("三点半发布", "3:30 发布"), (0, 5));
    }
}
