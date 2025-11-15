function proxyAttribute(url) {
    if (/^data:image\//.test(url)) {
        return url;
    } else {
        let hexUrl = url.split("").map(c => c.charCodeAt(0).toString(16).padStart(2, "0")).join("");
        return `/api/proxy/${hexUrl}`;
    }
}

function renderMarkdown(content, element) {
    insertScript("/static/js/third_party/purify.min.js");

    const HEADINGS = ["h1", "h2", "h3", "h4", "h5"];
    let containsCode = false;

    DOMPurify.addHook("afterSanitizeAttributes", (node) => {
        if (node.tagName === "IMG") {
            node.classList.add("ui");
            node.classList.add("image");

            if (node.hasAttribute("src")) {
                node.setAttribute("src", proxyAttribute(node.getAttribute("src")));
                node.setAttribute("loading", "lazy");
            }
        }

        if (HEADINGS.includes(node.tagName.toLowerCase())) {
            node.classList.add("ui");
            node.classList.add("header");
        }

        if (node.tagName === "CODE" && !containsCode) {
            containsCode = true;
        }
    });

    insertScript("/static/js/third_party/marked.min.js");

    let renderedContent = marked.parse(content);

    renderedContent = DOMPurify.sanitize(renderedContent, {
        ALLOWED_TAGS: HEADINGS.concat(["p", "ul", "ol", "li", "em", "strong", "italic", "code", "a", "blockquote", "pre", "hr", "img"]),
        KEEP_CONTENT: true
    });

    element.html(renderedContent);

    if (containsCode) {
        insertScript("/static/js/third_party/highlight.min.js");
        insertStyleSheet("/static/css/third_party/highlight.min.css");

        hljs.highlightAll();
    }
}

function loadReadme(username, repo, tree) {
    $.getJSON(`/api/repo/${username}/${repo}/tree/${tree}/readme`)
        .done((json) => {
            let fileName = json.file_name;
            const loweredFileName = fileName.toLowerCase();
            const isMarkdown = loweredFileName.endsWith(".md") || loweredFileName.endsWith(".markdown");

            let content = json.content;
            let readmeElement = $("#readme");

            if (isMarkdown) {
                renderMarkdown(content, readmeElement);
            } else {
                insertScript("/static/js/third_party/purify.min.js");

                readmeElement.html(DOMPurify.sanitize(content, {
                    ALLOWED_TAGS: [],
                    KEEP_CONTENT: true
                }));
            }

            $("#readme-file-name").html(DOMPurify.sanitize(fileName, {
                ALLOWED_TAGS: [],
                KEEP_CONTENT: true
            }));

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
