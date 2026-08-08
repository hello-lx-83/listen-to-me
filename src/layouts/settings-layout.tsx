import { NavLink, Outlet, useLocation } from "react-router-dom";

import { buttonVariants } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { cn } from "@/lib/utils";

const items = [
  { title: "通用", to: "/settings/general" },
  { title: "快捷键", to: "/settings/shortcut" },
  { title: "语音与语言", to: "/settings/speech" },
  { title: "模型与网络", to: "/settings/models" },
  { title: "隐私与数据", to: "/settings/privacy" },
  { title: "关于", to: "/settings/about" },
];

export function SettingsLayout() {
  const location = useLocation();

  return (
    <div className="flex min-h-full">
      <aside className="w-52 shrink-0 p-4">
        <p className="mb-3 px-2 text-xs font-medium text-muted-foreground">设置</p>
        <nav className="flex flex-col gap-1">
          {items.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              className={cn(
                buttonVariants({ variant: "ghost" }),
                "justify-start",
                location.pathname === item.to && "bg-accent",
              )}
            >
              {item.title}
            </NavLink>
          ))}
        </nav>
      </aside>
      <Separator orientation="vertical" />
      <div className="min-w-0 flex-1">
        <Outlet />
      </div>
    </div>
  );
}
