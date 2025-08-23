import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "react-hot-toast";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { FileInput } from "@/components/ui/file-input";
import * as Sentry from "@sentry/react";
import { readTextFile } from "@tauri-apps/plugin-fs";
import useStore from "@/store/store";

const getLogs = async () => {
  const logs = await invoke<string | null>("get_logs");
  return logs;
};

const deactivateHiding = async (value: boolean) => {
  await invoke("set_deactivate_hiding", { deactivate: value });
};

export function Report() {
  const [description, setDescription] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const user = useStore((state) => state.user);

  useEffect(() => {
    deactivateHiding(true);
    return () => {
      deactivateHiding(false);
    };
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!description.trim()) {
      toast.error("Please provide a description of the issue");
      return;
    }

    setIsSubmitting(true);
    try {
      let blob = await selectedFile?.arrayBuffer();

      if (user?.email) {
        Sentry.getCurrentScope().setUser({
          email: user.email.trim(),
        });
      }

      if (blob !== undefined) {
        Sentry.getCurrentScope().addAttachment({
          data: new Uint8Array(blob),
          filename: selectedFile ? selectedFile.name : "screenshot.png",
        });
      }

      Sentry.getCurrentScope().addAttachment({
        data: description,
        filename: "description.txt",
      });

      const logs = await getLogs();

      if (logs !== null) {
        const logs_content = await readTextFile(logs);
        Sentry.getCurrentScope().addAttachment({
          data: logs_content,
          filename: "logs.txt",
        });
      }

      const reportId = Math.random().toString(36).substring(7);
      Sentry.captureMessage(`User reported an issue (${reportId})`);

      Sentry.getCurrentScope().clearAttachments();
    } catch (error) {
      toast.error("Failed to submit report");
      console.error(error);
      return;
    }
    setIsSubmitting(false);
    toast.success("Report submitted successfully", {
      duration: 5000,
    });
  };

  return (
    <div className="flex flex-col p-6 max-w-2xl mx-auto">
      <h1 className="text-2xl font-semibold mb-4">Report an Issue</h1>
      <form onSubmit={handleSubmit} className="space-y-4">
        <div className="space-y-2">
          <label htmlFor="description" className="block text-sm font-medium">
            Description
          </label>
          <Textarea
            id="description"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Please describe the issue you're experiencing..."
            className="min-h-[150px]"
            required
          />
          <FileInput
            acceptedFileTypes="PNG, JPEG, JPG or MP4"
            maxSize={15}
            onFileSelect={(file) => {
              setSelectedFile(file);
            }}
          />
        </div>
        <Button type="submit" className="w-full" disabled={isSubmitting}>
          {isSubmitting ? "Submitting..." : "Submit Report"}
        </Button>
      </form>
    </div>
  );
}

export default Report;
