/**
 * Displays a notification in the top left
 *
 * @param type Either `info`, `success`, `warning` or `error`
 * @param message Message to be displayed to user
 */
function sendNotification(type, message) {
    $("body").toast({
        message: message,
        class: type,
        className: {
            toast: "ui message"
        },
        showProgress: "top"
    });
}

/**
 * Inserts a `<script>` into current documents `<head>`
 * @param url URL of the script to be inserted
 */
function insertScript(url) {
    const script = document.createElement("script");
    script.src = url;

    $("head").append(script);
}

/**
 * Inserts a CSS stylesheet into current documents `<head>`
 * @param url URL of the style sheet to be inserted
 */
function insertStyleSheet(url) {
    $("head").append(`<link rel="stylesheet" type="text/css" href="${url}">`);
}
