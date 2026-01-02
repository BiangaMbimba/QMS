"use client";

import { cn } from "@/lib/utils";
import { FileStack, LayoutDashboard, Monitor, Settings } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";

const SideBar = () => {
  // 4. On récupère le chemin actuel via le hook de Next.js
  const pathname = usePathname();

  const navItems = [
    { title: "Public Display", path: "/display", icon: Monitor },
    { title: "Dashboard", path: "/dashboard", icon: LayoutDashboard },
    { title: "Settings", path: "/settings", icon: Settings },
  ];

  return (
    <aside className="w-80 min-h-screen border-r border-sidebar-border flex flex-col bg-sidebar">
      {/* Header */}
      <div className="p-8 border-b border-sidebar-border">
        <div className="flex flex-row gap-4 items-center">
          <FileStack className="w-6 h-6 text-sidebar-foreground" /> {/* J'ai mis text-primary pour qu'on la voit mieux */}
          <div className="">
            <h1 className="font-bold text-lg text-sidebar-foreground">QSM</h1>
            <p className="text-xs text-sidebar-foreground/60">
              Management system
            </p>
          </div>
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex-1 p-4 space-y-2">
        {navItems.map((item) => {
          // 5. Comparaison simple pour voir si on est sur la page active
          const isActive = pathname === item.path;

          return (
            <Link
              key={item.path}
              href={item.path} // <--- 6. Dans Next.js, c'est 'href', pas 'to'
              className={cn(
                "flex items-center gap-3 px-4 py-3 rounded-lg transition-all duration-200",
                isActive
                  ? "bg-sidebar-accent text-sidebar-primary font-bold"
                  : "text-sidebar-foreground/70 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
              )}
            >
              <item.icon className="w-5 h-5" />
              <span className="font-medium">{item.title}</span>
            </Link>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="p-4 border-t border-sidebar-border">
        <div className="text-xs text-sidebar-foreground/50 text-center">
          v1.0.0 • Enterprise Edition
        </div>
      </div>
    </aside>
  );
};

export default SideBar;