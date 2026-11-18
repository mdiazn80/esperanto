import { HistoryIcon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { HistorySession_Serialize as HistorySession } from "@/bindings";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { HistoryDetail } from "@/features/history/history-detail";
import { HistoryList } from "@/features/history/history-list";
import { useHistory } from "@/store/history";

/** Toolbar button that opens past sessions. */
export function HistoryDialog() {
  const { t } = useTranslation();
  const load = useHistory((s) => s.load);
  const openSession = useHistory((s) => s.open);
  const [open, setOpen] = useState(false);
  const [session, setSession] = useState<HistorySession | null>(null);

  function onOpenChange(next: boolean) {
    setOpen(next);
    setSession(null);
    if (next) void load();
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <Tooltip>
        <TooltipTrigger asChild>
          <DialogTrigger asChild>
            <Button variant="ghost" size="icon" aria-label={t("history.title")}>
              <HistoryIcon />
            </Button>
          </DialogTrigger>
        </TooltipTrigger>
        <TooltipContent>{t("history.title")}</TooltipContent>
      </Tooltip>

      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("history.title")}</DialogTitle>
          <DialogDescription>{t("history.description")}</DialogDescription>
        </DialogHeader>
        {session ? (
          <HistoryDetail session={session} onBack={() => setSession(null)} />
        ) : (
          <HistoryList onOpen={(id) => void openSession(id).then(setSession)} />
        )}
      </DialogContent>
    </Dialog>
  );
}
