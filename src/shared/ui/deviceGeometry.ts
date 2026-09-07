/**
 * Geometry of the iPod touch (4th generation) illustration in viewBox units.
 * Overlays (finger hints, countdown rings, labels) anchor to these points and
 * convert to percentages so any rendered size lines up.
 */
export const deviceViewBox = { width: 240, height: 480 } as const;

export const deviceAnchors = {
  power: { x: 170, y: 11 },
  volumeUp: { x: 10, y: 122 },
  volumeDown: { x: 10, y: 152 },
  home: { x: 120, y: 388 },
  dock: { x: 120, y: 430 },
  camera: { x: 120, y: 44 },
} as const;

export const deviceScreen = {
  x: 29.5,
  y: 78,
  width: 181,
  height: 272,
} as const;

export type DeviceAnchor = keyof typeof deviceAnchors;

/** Anchor position as CSS percentages of the illustration box. */
export function anchorPercent(anchor: DeviceAnchor): {
  left: string;
  top: string;
} {
  const point = deviceAnchors[anchor];
  return {
    left: `${(point.x / deviceViewBox.width) * 100}%`,
    top: `${(point.y / deviceViewBox.height) * 100}%`,
  };
}
