import { ExternalLinkIcon } from "lucide-react";
import { commands, type ModelInfo } from "@/bindings";
import { Button } from "@/components/ui/button";
import { runCommand } from "@/lib/notify";
import { cn } from "@/lib/utils";

/** Opens the model's license page in the browser. */
export function LicenseLink({
  model,
  className,
}: {
  model: ModelInfo;
  className?: string;
}) {
  return (
    <Button
      variant="link"
      className={cn("h-auto p-0", className)}
      onClick={() => runCommand(commands.openModelLicense(model.id))}
    >
      {model.licenseName}
      <ExternalLinkIcon />
    </Button>
  );
}
