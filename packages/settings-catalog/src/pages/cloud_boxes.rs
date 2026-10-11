use crate::data::*;
use crate::json::opt;
use crate::rows::{row, section, Page, Section};

/// CDXC:AgentBox 2026-10-01 SEE-ALSO:
/// Settings > Cloud Boxes (apps/desktop/src/app/window/settings_modal/tabs/cloud_boxes.rs and
/// tabs/cloud-boxes.tsx (deleted 2026-10-01)). `agentboxDefaultLocation` is the page's one setting; the other rows are
/// keyed by UI ids so the search and Ghostex Help can find every part of the page.
pub(crate) fn page() -> Page {
    Page {
        id: "cloudBoxes",
        title: "Cloud Boxes",
        sections: vec![
            overview(),
            providers(),
            agent_sign_in(),
            new_threads(),
            boxes(),
            how_to(),
        ],
    }
}

pub(crate) fn overview() -> Section {
    section(
        "overview",
        "Cloud Boxes",
        vec![
            row("agentboxStatus", "agentbox", "Install agentbox, the free open-source command line tool Ghostex uses to run agent sessions in boxes, see its version and whether Docker is running, and run its check."),
            row("agentboxSetUpForMe", "Set it up for me", "Start an agent that installs agentbox, asks which clouds you want, creates the API tokens in your browser, and signs Claude and Codex in for boxes."),
            row("agentboxWhatIsABox", "What is a box?", "A box is an isolated copy of your project where an agent works without touching this computer. Your agent's settings, skills and Codex sign-in go with it, and its web app opens on this computer."),
        ],
    )
}

pub(crate) fn providers() -> Section {
    section(
        "providers",
        "Where boxes run",
        vec![
            row("agentboxProviders", "Where boxes run", "Set up Docker on this computer, or log in to a cloud provider with an API token and prepare its base image once. Cloud boxes bill while they exist.").options(&[opt("Docker on this computer", "docker"), opt("Hetzner", "hetzner"), opt("Vercel", "vercel"), opt("Daytona", "daytona"), opt("E2B", "e2b"), opt("DigitalOcean", "digitalocean")]),
            row("agentboxRemoteDocker", "Your own server (SSH)", "Add your own server with Docker by a name and its SSH address (user@host or a name from ~/.ssh/config), then check that boxes can run there."),
        ],
    )
}

pub(crate) fn agent_sign_in() -> Section {
    section(
        "agentSignIn",
        "Agent sign-in",
        vec![
            row("agentboxClaudeSignIn", "Claude in boxes", "Claude needs its own one-time sign-in for boxes, so Claude on this computer stays signed in. Every box uses it."),
            row("agentboxCodexSignIn", "Codex in boxes", "Boxes on this computer reuse your Codex sign-in. Sign in here for cloud boxes when Codex is not signed in."),
        ],
    )
}

pub(crate) fn new_threads() -> Section {
    section(
        "newThreads",
        "New threads",
        vec![row(
            "agentboxDefaultLocation",
            "Default location",
            "Where new threads run unless you pick another location.",
        )
        .options(AGENTBOX_DEFAULT_LOCATION_OPTIONS)],
    )
}

pub(crate) fn boxes() -> Section {
    section(
        "boxes",
        "Your boxes",
        vec![row(
            "agentboxBoxes",
            "Your boxes",
            "See every box, open a box's web app, stop a box, or destroy it to delete it for good.",
        )],
    )
}

pub(crate) fn how_to() -> Section {
    section(
        "howTo",
        "How it works",
        vec![
            row("agentboxStartThread", "Start a thread in a box", "Open New Thread and pick where it runs under Run on. Boxes open in the terminal view."),
            row("agentboxWebApp", "Open a box's web app", "Right-click the session and choose Open Box Web App to open it on this computer. Add an agentbox.yaml with services.web.expose.port to start your dev server in the box automatically."),
            row("agentboxBilling", "Cloud boxes bill until stopped", "Cloud providers charge while a box exists. Deleting a session stops its box and keeps its work; Destroy removes the box."),
        ],
    )
}
