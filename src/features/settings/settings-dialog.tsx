import { Settings2Icon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { ModelsSettings } from "@/features/models/models-settings";
import { ProviderSettings } from "@/features/providers/provider-settings";
import { AudioSettings } from "@/features/settings/audio-settings";
import { GeneralSettings } from "@/features/settings/general-settings";
import { PrivacySettings } from "@/features/settings/privacy-settings";

export function SettingsDialog() {
  const { t } = useTranslation();

  return (
    <Dialog>
      <Tooltip>
        <TooltipTrigger asChild>
          <DialogTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              aria-label={t("toolbar.openSettings")}
            >
              <Settings2Icon />
            </Button>
          </DialogTrigger>
        </TooltipTrigger>
        <TooltipContent>{t("toolbar.openSettings")}</TooltipContent>
      </Tooltip>

      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{t("settings.title")}</DialogTitle>
          <DialogDescription>{t("settings.description")}</DialogDescription>
        </DialogHeader>

        <Tabs defaultValue="general">
          <TabsList variant="line" className="w-full justify-start border-b">
            <TabsTrigger value="general">
              {t("settings.tabs.general")}
            </TabsTrigger>
            <TabsTrigger value="audio">{t("settings.tabs.audio")}</TabsTrigger>
            <TabsTrigger value="models">
              {t("settings.tabs.models")}
            </TabsTrigger>
            <TabsTrigger value="provider">
              {t("settings.tabs.provider")}
            </TabsTrigger>
            <TabsTrigger value="privacy">
              {t("settings.tabs.privacy")}
            </TabsTrigger>
          </TabsList>

          <TabsContent value="general" className="flex flex-col gap-6 pt-4">
            <GeneralSettings />
          </TabsContent>

          <TabsContent value="audio" className="flex flex-col gap-6 pt-4">
            <AudioSettings />
          </TabsContent>

          <TabsContent value="models" className="pt-4">
            <ModelsSettings />
          </TabsContent>

          <TabsContent value="provider" className="pt-4">
            <ProviderSettings />
          </TabsContent>

          <TabsContent value="privacy" className="flex flex-col gap-6 pt-4">
            <PrivacySettings />
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
