import { expect, test } from "@playwright/test";
import type { WithLabel } from "@/components/ui/button.story";

test("renders the label passed as a prop", async ({ mount }) => {
    const component = await mount<typeof WithLabel>("components/ui/button/WithLabel", { label: "Push changes" });

    await expect(component.getByRole("button")).toHaveText("Push changes");
});

test("clicking should report every click", async ({ mount }) => {
    const component = await mount("components/ui/button/CountsClicks");

    await component.getByRole("button").click();
    await component.getByRole("button").click();

    await expect(component.getByTestId("clicks")).toHaveValue("2");
});

test("renders a disabled button", async ({ mount }) => {
    const component = await mount("components/ui/button/Disabled");

    await expect(component.getByRole("button")).toBeDisabled();
});
