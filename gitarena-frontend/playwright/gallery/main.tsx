/// <reference types="vite/client" />
import { StrictMode, type ComponentType } from "react";
import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";

import "../../app/globals.css";

type StoryProps = Record<string, unknown>;
type StoryModule = Record<string, ComponentType<StoryProps>>;

const stories = import.meta.glob<StoryModule>("../../components/**/*.story.tsx");

// "../../components/ui/button.story.tsx" -> "components/ui/button"
function storyPath(file: string) {
    return file.replace(/^(\.\.\/)+/, "").replace(/\.story\.\w+$/, "");
}

async function resolve(story: string) {
    const separator = story.lastIndexOf("/");
    const path = story.slice(0, separator);
    const name = story.slice(separator + 1);
    const file = Object.keys(stories).find((candidate) => storyPath(candidate) === path || storyPath(candidate).endsWith("/" + path));
    const exports = file ? await stories[file]() : undefined;
    return exports?.[name] ?? exports?.default;
}

declare global {
    interface Window {
        mount: (params: { story: string; props?: StoryProps }) => Promise<void>;
        unmount: () => Promise<void>;
    }
}

const rootElement = document.getElementById("root")!;
let root: Root | undefined;

window.mount = async ({ story, props }) => {
    const Story = await resolve(story);
    if (!Story) {
        throw new Error(`Unknown story: ${story}`);
    }
    // Reuse the root so update() reconciles instead of remounting, preserving component state.
    root ??= createRoot(rootElement);
    // flushSync so a render error rejects this promise instead of being swallowed.
    flushSync(() => {
        root!.render(
            <StrictMode>
                <Story {...props} />
            </StrictMode>
        );
    });
};

window.unmount = async () => {
    root?.unmount();
    root = undefined;
};
