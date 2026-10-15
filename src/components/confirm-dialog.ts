import { mount } from "svelte";

import ConfirmDialog from "./ConfirmDialog.svelte";

interface ConfirmItem {
  id: string;
  label: string;
  sizeBytes: number;
  caution: boolean;
  paths: string[];
}

export interface ConfirmRequest {
  title: string;
  subtitle: string;
  items: ConfirmItem[];
  note?: string;
  confirmLabel: string;
}

export function confirmDialog(request: ConfirmRequest): Promise<boolean> {
  return new Promise((resolve) => {
    mount(ConfirmDialog, {
      target: document.body,
      props: {
        request,
        onresolve: (ok: boolean) => {
          resolve(ok);
        },
      },
    });
  });
}