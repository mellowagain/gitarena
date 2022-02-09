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
 * Writes `content` to clipboard
 *
 * @param content Content to be written to clipboard
 */
function writeClipboard(content) {
    navigator.clipboard.writeText(content).then(() => {
        sendNotification("success", "Copied to clipboard");
    }, () => {
        sendNotification("error", "Failed to copy to clipboard");
    });
}

/**
 * Inserts a `<script>` into current documents `<head>`
 *
 * @param url URL of the script to be inserted
 */
function insertScript(url) {
    const script = document.createElement("script");
    script.src = url;

    $("head").append(script);
}

/**
 * Inserts a CSS stylesheet into current documents `<head>`
 *
 * @param url URL of the style sheet to be inserted
 */
function insertStyleSheet(url) {
    $("head").append(`<link rel="stylesheet" type="text/css" href="${url}">`);
}

document.addEventListener("DOMContentLoaded", () => {
    htmx.onLoad(() => {
        $(".popup").popup();

        $(".copy.button").off().click(function () {
            writeClipboard($(this).attr("data-copy"));
        });

        $("#user-popup").dropdown();
    });
});

function displayHtmxError(event) {
    sendNotification("error", "Error occurred while sending request");
    console.error(event);
}

document.addEventListener("htmx:responseError", displayHtmxError);
document.addEventListener("htmx:sendError", displayHtmxError);
