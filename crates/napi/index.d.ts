export interface InputEvent {
  type: 'KeyDown' | 'KeyUp' | 'MouseMove' | 'MouseDown' | 'MouseUp' | 'Wheel'
  key?: string
  button?: 'Left' | 'Right' | 'Middle'
  x?: number
  y?: number
  deltaX?: number
  deltaY?: number
  /** Unix timestamp in milliseconds */
  timestamp: number
}

/**
 * Start listening for global input events.
 * Calls `callback` for every keyboard / mouse event.
 * Throws if already listening — call `stopListening()` first.
 *
 * On Linux Wayland the native bridge must be running (`rinhook.service`).
 * On all other platforms rdev is used directly.
 */
export function startListening(callback: (event: InputEvent) => void): void

/** Stop listening. Safe to call when not listening. */
export function stopListening(): void
