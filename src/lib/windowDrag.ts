import { getCurrentWindow } from "@tauri-apps/api/window";

type WindowDragRequest = {
  button: number;
  defaultPrevented: boolean;
  preventDefault: () => void;
  startDragging: () => Promise<void>;
};

export async function startWindowDragIfNeeded({
  button,
  defaultPrevented,
  preventDefault,
  startDragging,
}: WindowDragRequest): Promise<boolean> {
  if (button !== 0 || defaultPrevented) {
    return false;
  }

  preventDefault();
  await startDragging();
  return true;
}

export async function handleWindowDragMouseDown(
  event: Pick<MouseEvent, "button" | "defaultPrevented" | "preventDefault">,
): Promise<void> {
  try {
    await startWindowDragIfNeeded({
      button: event.button,
      defaultPrevented: event.defaultPrevented,
      preventDefault: () => {
        event.preventDefault();
      },
      startDragging: async () => {
        await getCurrentWindow().startDragging();
      },
    });
  } catch (error) {
    console.error("Failed to start window drag.", error);
  }
}
