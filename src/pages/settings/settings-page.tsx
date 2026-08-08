import type { PropsWithChildren } from "react";

import { PageHeader } from "@/components/app/page-header";

export function SettingsPage({ title, description, children }: PropsWithChildren<{ title: string; description: string }>) {
  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 p-8">
      <PageHeader title={title} description={description} />
      {children}
    </div>
  );
}
