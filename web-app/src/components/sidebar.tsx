import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import { HiHome, HiCog6Tooth, HiUserGroup, HiArrowRightStartOnRectangle } from "react-icons/hi2";
import Logo from "@/assets/Hopp.png";
import { Button } from "./ui/button";
import { resetAllStores, useHoppStore } from "@/store/store";

const items = [
  {
    title: "Dashboard",
    url: "/dashboard",
    icon: HiHome,
  },
  {
    title: "Settings",
    url: "/settings",
    icon: HiCog6Tooth,
  },
  {
    title: "Teammates",
    url: "/teammates",
    icon: HiUserGroup,
  },
];

export function HoppSidebar() {
  const setAuthToken = useHoppStore((state) => state.setAuthToken);

  return (
    <Sidebar className="px-1 py-3 bg-sidebar">
      <SidebarHeader>
        <img src={Logo} alt="Hopp Logo" className="mr-auto h-[40px]" />
      </SidebarHeader>
      <SidebarContent>
        <SidebarGroup>
          <SidebarMenu>
            {items.map((item) => (
              <SidebarMenuItem key={item.title}>
                <SidebarMenuButton asChild>
                  <a href={item.url}>
                    <item.icon />
                    <span>{item.title}</span>
                  </a>
                </SidebarMenuButton>
              </SidebarMenuItem>
            ))}
          </SidebarMenu>
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter>
        <Button
          variant="outline"
          className="w-full flex flex-row justify-start max-w-min items-start gap-2"
          onClick={() => {
            resetAllStores();
            setAuthToken(null);
          }}
        >
          <HiArrowRightStartOnRectangle /> Logout
        </Button>
      </SidebarFooter>
    </Sidebar>
  );
}
