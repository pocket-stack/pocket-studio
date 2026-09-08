/** Grid metrics shared by the store pages so rows page to the viewport. */
export const CARD_GAP = 14;
/** Cards run a little wider than their artwork so names get room. */
export const CARD_PAD = 16;
/** Name, meta line and the action pill under the artwork. */
export const CARD_CHROME = 66;
export const SECTION_HEADER = 28;
export const HERO_MIN = 104;
export const HERO_MAX = 200;
export const TILE_ROW = 84;
/** Artwork shrinks a step at the minimum window so two shelves still fit. */
export function artworkSize(height: number): number {
  return height < 560 ? 72 : 88;
}
