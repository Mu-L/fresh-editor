import { Editor, Command } from "fresh";

interface PluginConfig {
    greeting: string;
    showNotification: boolean;
}

export function activate(editor: Editor) {
    const config: PluginConfig = {
        greeting: "Hello from Fresh!",
        showNotification: true,
    };

    editor.registerCommand("hello", () => {
        if (config.showNotification) {
            editor.notify(config.greeting);
        }
    });

    editor.registerCommand("insert-date", () => {
        const date = new Date().toISOString();
        editor.insertText(date);
    });
}
