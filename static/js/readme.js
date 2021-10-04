function proxyAttribute(url) {
    if (/^data:image\//.test(url)) {
        return url;
    } else {
        let hexUrl = url.split("").map(c => c.charCodeAt(0).toString(16).padStart(2, "0")).join("");
        return `/api/proxy/${hexUrl}`;
    }
}

function loadReadme(username, repo, tree) {
    $.getJSON(`/api/repo/${username}/${repo}/tree/${tree}/readme`)
        .done((json) => {
            insertScript("/static/js/third_party/purify.min.js");

            DOMPurify.addHook("afterSanitizeAttributes", (node) => {
                if (node.tagName === "IMG" && node.hasAttribute("src")) {
                    node.setAttribute("src", proxyAttribute(node.getAttribute("src")));
                }
            });

            let fileName = json.file_name;
            const loweredFileName = fileName.toLowerCase();

            let content = json.content;

            const isMarkdown = loweredFileName.endsWith(".md") || loweredFileName.endsWith(".markdown");
            const allowedTags = isMarkdown ? ["h1", "h2", "h3", "h4", "h5", "p", "ul", "ol", "li", "em", "strong", "italic", "code", "a", "blockquote", "pre", "hr", "img"] : [];

            if (isMarkdown) {
                insertScript("/static/js/third_party/marked.min.js");
                content = marked(content);
            }

            fileName = DOMPurify.sanitize(fileName, {
                ALLOWED_TAGS: [],
                KEEP_CONTENT: true
            });

            content = DOMPurify.sanitize(content, {
                ALLOWED_TAGS: allowedTags,
                KEEP_CONTENT: true
            });

            $("#readme-file-name").html(fileName);

            const readmeElement = $("#readme");

            readmeElement.html(content);

            if (isMarkdown) {
                $("#readme img").addClass("ui image");

                for (let i = 1; i < 6; i++) {
                    $(`#readme h${i}`).addClass("ui header");
                }

                if ($("#readme code").length > 0) {
                    insertScript("/static/js/third_party/highlight.min.js");
                    insertStyleSheet("/static/css/third_party/highlight.min.css");

                    hljs.highlightAll();
                }
            }

            readmeElement.removeClass("ui fluid placeholder");
        })
        .fail((xhr, _status, _httpMessage) => {
            if (xhr.status === 404) {
                $("#readme-parent").addClass("hidden");
            } else {
                sendNotification("error", "Failed to load README");
            }
        });
}
