/**
 * Tauri v2 Updater Controller
 */

export interface UpdateInfo {
  available: boolean;
  version?: string;
  currentVersion?: string;
  body?: string;
  date?: string;
}

export interface DownloadEvent {
  event: "Started" | "Progress" | "Finished";
  data?: {
    contentLength?: number;
    chunkLength?: number;
  };
}

export async function checkForAppUpdates(): Promise<{
  updateAvailable: boolean;
  version?: string;
  currentVersion: string;
  notes?: string;
  error?: string;
}> {
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check();

    if (update?.available) {
      return {
        updateAvailable: true,
        version: update.version,
        currentVersion: update.currentVersion,
        notes: update.body ?? "Performance optimizations and stability improvements.",
      };
    } else {
      return {
        updateAvailable: false,
        currentVersion: update?.currentVersion ?? "0.1.0",
      };
    }
  } catch (err: unknown) {
    console.warn("Tauri updater check:", err);
    return {
      updateAvailable: false,
      currentVersion: "0.1.0",
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

export async function installAppUpdate(
  onProgress?: (downloaded: number, total?: number) => void
): Promise<{ success: boolean; message?: string }> {
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check();

    if (!update?.available) {
      return { success: false, message: "No update available to install." };
    }

    let downloadedBytes = 0;
    let totalBytes: number | undefined;

    await update.downloadAndInstall((event: DownloadEvent) => {
      switch (event.event) {
        case "Started":
          totalBytes = event.data?.contentLength;
          break;
        case "Progress":
          downloadedBytes += event.data?.chunkLength ?? 0;
          onProgress?.(downloadedBytes, totalBytes);
          break;
        case "Finished":
          break;
      }
    });

    return { success: true };
  } catch (err: unknown) {
    console.error("Failed to download and install update:", err);
    return {
      success: false,
      message: err instanceof Error ? err.message : String(err),
    };
  }
}
