const RULES = [
    {
        label: "bug",
        keywords: [
            "bug",
            "crash",
            "broken",
            "error",
            "fail",
            "exception",
            "panic",
            "wrong",
            "incorrect",
            "regression",
        ],
    },
    {
        label: "feature",
        keywords: [
            "feature",
            "request",
            "add",
            "support",
            "implement",
            "enhancement",
            "would be nice",
            "suggestion",
        ],
    },
    {
        label: "documentation",
        keywords: [
            "doc",
            "docs",
            "readme",
            "comment",
            "example",
            "guide",
            "typo",
            "spelling",
        ],
    },
    {
        label: "question",
        keywords: [
            "how to",
            "how do",
            "question",
            "help",
            "clarify",
            "confused",
            "what is",
            "why does",
        ],
    },
    {
        label: "performance",
        keywords: [
            "slow",
            "performance",
            "memory",
            "leak",
            "speed",
            "lag",
            "latency",
            "cpu",
            "ram",
        ],
    },
    {
        label: "security",
        keywords: [
            "security",
            "vulnerability",
            "cve",
            "exploit",
            "injection",
            "auth",
            "permission",
        ],
    },
];

// Word-boundary aware match — avoids "doc" matching "docker"
function matchesKeyword(text, keyword) {
    const escaped = keyword.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const pattern = keyword.includes(" ")
        ? new RegExp(escaped, "i")
        : new RegExp(`\\b${escaped}\\b`, "i");
    return pattern.test(text);
}

function classifyIssue(title, body) {
    const text = `${title} ${body}`;
    return RULES.filter((rule) =>
        rule.keywords.some((kw) => matchesKeyword(text, kw)),
    ).map((rule) => rule.label);
}

async function applyLabels(repo, issueNumber, labels) {
    const url = `https://api.github.com/repos/${repo}/issues/${issueNumber}/labels`;
    const res = await fetch(url, {
        method: "POST",
        headers: {
            Authorization: `Bearer ${process.env.GITHUB_TOKEN}`,
            "Content-Type": "application/json",
            Accept: "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
        },
        body: JSON.stringify({ labels }),
    });

    if (!res.ok) {
        const err = await res.text();
        throw new Error(`GitHub API error ${res.status}: ${err}`);
    }

    return res.json();
}

async function main() {
    const { ISSUE_TITLE, ISSUE_BODY, ISSUE_NUMBER, REPO, GITHUB_TOKEN } =
        process.env;

    if (!GITHUB_TOKEN) throw new Error("GITHUB_TOKEN is not set");
    if (!ISSUE_NUMBER) throw new Error("ISSUE_NUMBER is not set");
    if (!REPO) throw new Error("REPO is not set");

    const title = ISSUE_TITLE ?? "";
    const body = ISSUE_BODY ?? "";

    console.log(`Processing issue #${ISSUE_NUMBER}: "${title}"`);

    const matched = classifyIssue(title, body);
    const labels = matched.length > 0 ? matched : ["needs-triage"];

    console.log(`Matched labels: ${labels.join(", ")}`);

    await applyLabels(REPO, ISSUE_NUMBER, labels);

    console.log(
        `Successfully applied ${labels.length} label(s) to #${ISSUE_NUMBER}`,
    );
}

main().catch((err) => {
    console.error(`Error: ${err.message}`);
    process.exit(1); // fails the workflow step visibly
});
