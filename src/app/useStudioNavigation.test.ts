import { ref } from "vue";
import { describe, expect, it } from "vitest";
import { createStudioNavigation } from "./useStudioNavigation";

describe("desktop navigation history", () => {
  it("disables both directions at startup and respects history boundaries", () => {
    const navigation = createStudioNavigation();
    navigation.back();
    navigation.forward();
    expect(navigation.current.value).toEqual({
      view: "device",
      section: "summary",
    });
    expect(navigation.canGoBack.value).toBe(false);
    expect(navigation.canGoForward.value).toBe(false);
  });

  it("restores device subpages and application details in both directions", () => {
    const navigation = createStudioNavigation();
    navigation.navigate({ view: "device", section: "conditions" });
    navigation.navigate({ view: "store", packageId: null });
    navigation.navigate({ view: "store", packageId: "pocket-reader" });
    navigation.back();
    expect(navigation.current.value).toEqual({
      view: "store",
      packageId: null,
    });
    navigation.back();
    expect(navigation.current.value).toEqual({
      view: "device",
      section: "conditions",
    });
    navigation.forward();
    navigation.forward();
    expect(navigation.current.value).toEqual({
      view: "store",
      packageId: "pocket-reader",
    });
    expect(navigation.canGoForward.value).toBe(false);
  });

  it("starts a new branch when navigating after going back", () => {
    const navigation = createStudioNavigation();
    navigation.navigate({ view: "store", packageId: null });
    navigation.navigate({ view: "installed" });
    navigation.back();
    navigation.navigate({ view: "logs" });
    expect(navigation.canGoForward.value).toBe(false);
    navigation.back();
    expect(navigation.current.value).toEqual({
      view: "store",
      packageId: null,
    });
    navigation.forward();
    expect(navigation.current.value).toEqual({ view: "logs" });
  });

  it("does not add duplicate entries when the current page is selected again", () => {
    const navigation = createStudioNavigation();
    navigation.navigate({ view: "installed" });
    navigation.navigate({ view: "installed" });
    navigation.back();
    expect(navigation.current.value.view).toBe("device");
    expect(navigation.canGoBack.value).toBe(false);
    navigation.navigate({ view: "device", section: "summary" });
    expect(navigation.canGoForward.value).toBe(true);
  });

  it("locks all navigation during preparation or a modal without consuming history", () => {
    const locked = ref(false);
    const navigation = createStudioNavigation(() => !locked.value);
    navigation.navigate({ view: "logs" });
    locked.value = true;
    navigation.back();
    navigation.navigate({ view: "installed" });
    expect(navigation.current.value.view).toBe("logs");
    expect(navigation.canGoBack.value).toBe(false);
    locked.value = false;
    navigation.back();
    locked.value = true;
    navigation.forward();
    expect(navigation.current.value.view).toBe("device");
    expect(navigation.canGoForward.value).toBe(false);
    locked.value = false;
    expect(navigation.canGoForward.value).toBe(true);
    navigation.forward();
    expect(navigation.current.value.view).toBe("logs");
  });
});
