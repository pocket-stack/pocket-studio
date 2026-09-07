/**
 * Land initial focus on the field marked `autofocus`, else on the dialog
 * itself. `showModal()` would otherwise pick the first focusable control,
 * which is the close button in the header.
 */
export function focusInitial(dialog: HTMLDialogElement | null): void {
  if (!dialog) return;
  const target = dialog.querySelector<HTMLElement>("[autofocus]");
  (target ?? dialog).focus({ preventScroll: true });
}
