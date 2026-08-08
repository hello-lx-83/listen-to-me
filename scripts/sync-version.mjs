import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packagePath = path.join(repositoryRoot, "package.json");
const cargoPath = path.join(repositoryRoot, "src-tauri", "Cargo.toml");

const packageMetadata = JSON.parse(await readFile(packagePath, "utf8"));
const version = packageMetadata.version;

if (typeof version !== "string" || !/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error(`package.json contains an invalid version: ${String(version)}`);
}

const cargoManifest = await readFile(cargoPath, "utf8");
const synchronizedManifest = cargoManifest.replace(
  /^(\[package\][\s\S]*?^version\s*=\s*")[^"]+("\s*$)/m,
  `$1${version}$2`,
);

if (synchronizedManifest === cargoManifest && !cargoManifest.includes(`version = "${version}"`)) {
  throw new Error("Could not locate the package version in src-tauri/Cargo.toml");
}

if (synchronizedManifest !== cargoManifest) {
  await writeFile(cargoPath, synchronizedManifest, "utf8");
}

console.log(`Version synchronized: ${version}`);
