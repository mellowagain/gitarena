import { expect, test } from "@playwright/test";
import type { Controlled } from "@/components/ui/sliding-tab-bar.story";

test("clicking a tab should select it", async ({ mount }) => {
    const component = await mount("components/ui/sliding-tab-bar/Stateful");

    await component.getByRole("tab", { name: "Issues" }).click();

    await expect(component.getByTestId("active")).toHaveValue("issues");
    await expect(component.getByRole("tab", { name: "Issues" })).toHaveAttribute("aria-selected", "true");
});

test("changing the active prop should move the selection", async ({ mount }) => {
    const component = await mount<typeof Controlled>("components/ui/sliding-tab-bar/Controlled", { active: "code" });

    await expect(component.getByRole("tab", { name: "Code" })).toHaveAttribute("aria-selected", "true");

    await component.update({ active: "pulls" });

    await expect(component.getByRole("tab", { name: "Pull requests" })).toHaveAttribute("aria-selected", "true");
    await expect(component.getByRole("tab", { name: "Code" })).toHaveAttribute("aria-selected", "false");
});
