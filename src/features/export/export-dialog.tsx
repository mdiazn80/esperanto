import { DownloadIcon, Loader2Icon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { commands, type ExportContent, type ExportFormat } from "@/bindings";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { formatDateTime } from "@/lib/format";
import { notifyError } from "@/lib/notify";

const FORMATS: readonly ExportFormat[] = ["txt", "md", "srt", "vtt"];
const CONTENTS: readonly ExportContent[] = ["original", "translation", "both"];

/** Last choices, kept while the app runs. */
let lastFormat: ExportFormat = "txt";
let lastContent: ExportContent = "both";

/** Exports a session (current or saved) to a file the user picks. */
export function ExportDialog({
  historyId,
  startedAt,
  hasTranslation,
  size = "sm",
}: {
  /** Size of the button that opens it. */
  size?: "sm" | "lg";
  historyId: string;
  /** Unix ms, for the suggested file name. */
  startedAt: number;
  hasTranslation: boolean;
}) {
  const { t, i18n } = useTranslation();
  const [open, setOpen] = useState(false);
  const [format, setFormat] = useState<ExportFormat>(lastFormat);
  const [content, setContent] = useState<ExportContent>(lastContent);
  const [busy, setBusy] = useState(false);

  async function onSave() {
    lastFormat = format;
    lastContent = content;
    setBusy(true);
    try {
      const fileName = `${t("app.name")} ${formatDateTime(startedAt, i18n.language)}`;
      const path = await commands.exportSession(
        historyId,
        format,
        hasTranslation ? content : "original",
        fileName,
      );
      if (path) {
        toast.success(t("export.saved", { name: path.split("/").pop() }));
        setOpen(false);
      }
    } catch (error) {
      notifyError(error);
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline" size={size}>
          <DownloadIcon />
          {t("export.open")}
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{t("export.title")}</DialogTitle>
          <DialogDescription>{t("export.description")}</DialogDescription>
        </DialogHeader>

        <fieldset className="flex flex-col gap-2">
          <legend className="mb-2 text-sm font-medium">
            {t("export.format")}
          </legend>
          <RadioGroup
            value={format}
            onValueChange={(value) => setFormat(value as ExportFormat)}
          >
            {FORMATS.map((option) => (
              <div key={option} className="flex items-center gap-2">
                <RadioGroupItem id={`export-${option}`} value={option} />
                <Label htmlFor={`export-${option}`} className="font-normal">
                  {t(`export.formats.${option}`)}
                </Label>
              </div>
            ))}
          </RadioGroup>
        </fieldset>

        {hasTranslation && (
          <fieldset className="flex flex-col gap-2">
            <legend className="mb-2 text-sm font-medium">
              {t("export.content")}
            </legend>
            <RadioGroup
              value={content}
              onValueChange={(value) => setContent(value as ExportContent)}
            >
              {CONTENTS.map((option) => (
                <div key={option} className="flex items-center gap-2">
                  <RadioGroupItem
                    id={`export-content-${option}`}
                    value={option}
                  />
                  <Label
                    htmlFor={`export-content-${option}`}
                    className="font-normal"
                  >
                    {t(`export.contents.${option}`)}
                  </Label>
                </div>
              ))}
            </RadioGroup>
          </fieldset>
        )}

        <DialogFooter>
          <Button disabled={busy} onClick={() => void onSave()}>
            {busy && <Loader2Icon className="animate-spin" />}
            {t("export.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
