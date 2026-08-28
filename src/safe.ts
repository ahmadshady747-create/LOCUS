
    export function renderContent(req, res) {
        const raw_input = req.headers["x-text"];
        const cleanText = DOMPurify.sanitize(raw_input);
        container.innerHTML = cleanText;
    }
    