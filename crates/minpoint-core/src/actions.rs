//! Central action registry. Every MinPoint feature that can be triggered from the
//! context menu, a shortcut, a mouse button, a gesture, the launcher, a macro or the
//! tray is described here once.

use crate::{devtools::DevOp, text::TextOp, writing::WritingOp};
use serde::{Deserialize, Serialize};

/// Where an action makes sense.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ActionContext {
    /// Text is selected in the focused app.
    Selection,
    /// Caret is in an editable field (with or without selection).
    Editable,
    /// Always available (launcher, screenshots, …).
    Anywhere,
}

/// Capabilities a platform adapter reports at runtime. An action is only offered
/// when every capability it needs is actually available on this machine.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ReadSelection,
    ReplaceSelection,
    InsertText,
    Clipboard,
    ClipboardHistory,
    ScreenCapture,
    Ocr,
    Tts,
    WindowManagement,
    Microphone,
    LocalTranscription,
    AiConnection,
    AppLaunch,
    Macros,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Accessibility,
    InputMonitoring,
    ScreenRecording,
    Microphone,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputBehavior {
    /// Show the floating preview; user decides to replace/copy.
    Preview,
    /// Put the result on the clipboard.
    Copy,
    /// Open an MinPoint surface (palette, clipboard picker, OCR result…).
    OpenSurface,
    /// Side effect only (speak, snap window…).
    None,
}

/// Security classification used by the policy layer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SecurityClass {
    /// Pure local transformation of text the user selected.
    LocalText,
    /// Sends user-selected text to the connected ChatGPT account.
    ExternalAi,
    /// Reads screen pixels after explicit user action.
    ScreenRead,
    /// Synthesises input events / changes windows.
    InputSynthesis,
    /// Launches apps / URLs.
    Launch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "op", rename_all = "snake_case")]
pub enum ActionKind {
    Writing(WritingOp),
    Text(TextOp),
    Dev(DevOp),
    Builtin(Builtin),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Builtin {
    Copy,
    CopyPlainText,
    PastePlainText,
    SaveAsSnippet,
    InsertSnippet,
    ClipboardHistory,
    InsertDate,
    InsertTime,
    Calculate,
    ReadAloud,
    StopSpeaking,
    WriteWithAi,
    QuickLauncher,
    ScreenTextCapture,
    Screenshot,
    OpenApp,
    TextStats,
    UuidInsert,
    WindowLeftHalf,
    WindowRightHalf,
    WindowMaximize,
    WindowCenter,
    WindowTopLeft,
    WindowTopRight,
    WindowBottomLeft,
    WindowBottomRight,
    WindowNextDisplay,
    StartVoiceTyping,
    PauseMinPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActionDef {
    pub id: String,
    pub name: String,
    pub description: String,
    /// lucide icon name used by the UI.
    pub icon: String,
    /// Group for submenus (e.g. "tone", "case"). Empty = top level.
    pub group: String,
    pub contexts: Vec<ActionContext>,
    pub capabilities: Vec<Capability>,
    pub permissions: Vec<Permission>,
    pub offline: bool,
    pub requires_ai: bool,
    pub output: OutputBehavior,
    pub security: SecurityClass,
    pub kind: ActionKind,
}

struct B {
    d: ActionDef,
}

impl B {
    fn new(id: &str, name: &str, icon: &str, kind: ActionKind) -> Self {
        let (offline, requires_ai, security, caps, output) = match &kind {
            ActionKind::Writing(_) => (false, true, SecurityClass::ExternalAi, vec![Capability::AiConnection], OutputBehavior::Preview),
            ActionKind::Text(_) | ActionKind::Dev(_) => (true, false, SecurityClass::LocalText, vec![], OutputBehavior::Preview),
            ActionKind::Builtin(_) => (true, false, SecurityClass::LocalText, vec![], OutputBehavior::None),
        };
        B {
            d: ActionDef {
                id: id.into(),
                name: name.into(),
                description: String::new(),
                icon: icon.into(),
                group: String::new(),
                contexts: vec![ActionContext::Selection],
                capabilities: caps,
                permissions: vec![],
                offline,
                requires_ai,
                output,
                security,
                kind,
            },
        }
    }
    fn desc(mut self, s: &str) -> Self {
        self.d.description = s.into();
        self
    }
    fn group(mut self, g: &str) -> Self {
        self.d.group = g.into();
        self
    }
    fn ctx(mut self, c: &[ActionContext]) -> Self {
        self.d.contexts = c.to_vec();
        self
    }
    fn caps(mut self, c: &[Capability]) -> Self {
        self.d.capabilities.extend_from_slice(c);
        self
    }
    fn perms(mut self, p: &[Permission]) -> Self {
        self.d.permissions = p.to_vec();
        self
    }
    fn out(mut self, o: OutputBehavior) -> Self {
        self.d.output = o;
        self
    }
    fn ai(mut self) -> Self {
        self.d.requires_ai = true;
        self.d.offline = false;
        self.d.security = SecurityClass::ExternalAi;
        self
    }
    fn sec(mut self, s: SecurityClass) -> Self {
        self.d.security = s;
        self
    }
}

use ActionContext::*;
use Capability as C;

fn writing(id: &str, name: &str, icon: &str, op: WritingOp, group: &str) -> ActionDef {
    B::new(id, name, icon, ActionKind::Writing(op))
        .desc(op.summary())
        .group(group)
        .caps(&[C::ReadSelection])
        .perms(&[Permission::Accessibility])
        .d
}

fn text(id: &str, name: &str, icon: &str, op: TextOp, group: &str) -> ActionDef {
    B::new(id, name, icon, ActionKind::Text(op)).group(group).caps(&[C::ReadSelection]).perms(&[Permission::Accessibility]).d
}

fn dev(id: &str, name: &str, icon: &str, op: DevOp) -> ActionDef {
    B::new(id, name, icon, ActionKind::Dev(op)).group("dev").caps(&[C::ReadSelection]).perms(&[Permission::Accessibility]).d
}

fn builtin(id: &str, name: &str, icon: &str, b: Builtin) -> B {
    B::new(id, name, icon, ActionKind::Builtin(b))
}

/// All built-in actions in their default order.
pub fn builtin_actions() -> Vec<ActionDef> {
    use WritingOp as W;
    let sel_perm = [Permission::Accessibility];
    let win = |id: &str, name: &str, icon: &str, b: Builtin| {
        builtin(id, name, icon, b)
            .group("window")
            .ctx(&[Anywhere])
            .caps(&[C::WindowManagement])
            .perms(&sel_perm)
            .sec(SecurityClass::InputSynthesis)
            .d
    };
    vec![
        // ---- Selection: AI writing ----
        writing("ai.polish", "Polish Text", "sparkles", W::Polish, ""),
        writing("ai.paraphrase", "Paraphrase", "repeat", W::Paraphrase, ""),
        writing("ai.grammar", "Fix Grammar", "spell-check", W::FixGrammar, ""),
        writing("ai.spelling", "Fix Spelling", "spell-check-2", W::FixSpelling, ""),
        writing("ai.shorten", "Shorten", "shrink", W::Shorten, ""),
        writing("ai.expand", "Expand", "expand", W::Expand, ""),
        writing("ai.simplify", "Simplify", "feather", W::Simplify, ""),
        writing("ai.professional", "Make Professional", "briefcase", W::Professional, "tone"),
        writing("ai.casual", "Make Casual", "coffee", W::Casual, "tone"),
        writing("ai.friendly", "Make Friendly", "face-slightly-smiling", W::Friendly, "tone"),
        writing("ai.formal", "Make Formal", "landmark", W::Formal, "tone"),
        writing("ai.summarize", "Summarize", "list", W::Summarize, ""),
        writing("ai.explain", "Explain", "lightbulb", W::Explain, ""),
        writing("ai.translate", "Translate", "languages", W::Translate, ""),
        writing("ai.email", "Convert to Email", "mail", W::Email, "convert"),
        writing("ai.slack", "Convert to Slack Message", "message-square", W::Slack, "convert"),
        writing("ai.technical", "Technical Writing", "file-code", W::Technical, "convert"),
        writing("ai.docs", "Improve Documentation", "book-open", W::Documentation, "convert"),
        writing("ai.explain_code", "Explain Code", "code", W::ExplainCode, "dev"),
        writing("ai.explain_error", "Explain Error", "bug", W::ExplainError, "dev"),
        writing("ai.readable_code", "Improve Code Readability", "wand", W::ImproveCode, "dev"),
        writing("ai.document_code", "Generate Documentation", "file-text", W::DocumentCode, "dev"),
        // ---- Selection: local ----
        builtin("tts.read", "Read Aloud", "volume-2", Builtin::ReadAloud)
            .ctx(&[Selection])
            .caps(&[C::ReadSelection, C::Tts])
            .perms(&sel_perm)
            .desc("Speak the selection with a local system voice.")
            .d,
        builtin("calc.selection", "Calculate", "calculator", Builtin::Calculate)
            .ctx(&[Selection])
            .caps(&[C::ReadSelection])
            .perms(&sel_perm)
            .out(OutputBehavior::Preview)
            .desc("Evaluate the selected expression or unit conversion locally.")
            .d,
        text("case.upper", "UPPERCASE", "case-upper", TextOp::Uppercase, "case"),
        text("case.lower", "lowercase", "case-lower", TextOp::Lowercase, "case"),
        text("case.title", "Title Case", "heading-1", TextOp::TitleCase, "case"),
        text("case.sentence", "Sentence case", "case-sensitive", TextOp::SentenceCase, "case"),
        text("case.camel", "camelCase", "code", TextOp::CamelCase, "case"),
        text("case.pascal", "PascalCase", "code", TextOp::PascalCase, "case"),
        text("case.snake", "snake_case", "code", TextOp::SnakeCase, "case"),
        text("case.kebab", "kebab-case", "code", TextOp::KebabCase, "case"),
        text("case.constant", "CONSTANT_CASE", "code", TextOp::ConstantCase, "case"),
        text("text.trim", "Remove Extra Whitespace", "eraser", TextOp::TrimWhitespace, "text"),
        text("text.dedupe", "Remove Duplicate Lines", "list-x", TextOp::RemoveDuplicateLines, "text"),
        text("text.sort", "Sort Lines", "list-sort-ascending", TextOp::SortLines, "text"),
        text("text.reverse", "Reverse Text", "undo-2", TextOp::ReverseText, "text"),
        text("text.join", "Remove Line Breaks", "text-wrap", TextOp::RemoveLineBreaks, "text"),
        builtin("text.stats", "Count Words & Characters", "hash", Builtin::TextStats)
            .group("text")
            .ctx(&[Selection])
            .caps(&[C::ReadSelection])
            .perms(&sel_perm)
            .out(OutputBehavior::Preview)
            .d,
        dev("dev.json_format", "Format JSON", "braces", DevOp::JsonFormat),
        dev("dev.json_minify", "Minify JSON", "braces", DevOp::JsonMinify),
        dev("dev.json_validate", "Validate JSON", "badge-check", DevOp::JsonValidate),
        dev("dev.xml_format", "Format XML", "code-xml", DevOp::XmlFormat),
        dev("dev.b64_encode", "Base64 Encode", "binary", DevOp::Base64Encode),
        dev("dev.b64_decode", "Base64 Decode", "binary", DevOp::Base64Decode),
        dev("dev.url_encode", "URL Encode", "link", DevOp::UrlEncode),
        dev("dev.url_decode", "URL Decode", "link", DevOp::UrlDecode),
        dev("dev.sha256", "SHA-256 Hash", "fingerprint-pattern", DevOp::Sha256),
        dev("dev.ts_to_date", "Timestamp → Date", "clock", DevOp::TimestampToDate),
        dev("dev.color", "Convert Colour", "palette", DevOp::ColorConvert),
        dev("dev.csv_md", "CSV → Markdown Table", "table", DevOp::CsvToMarkdownTable),
        builtin("clip.copy", "Copy", "copy", Builtin::Copy)
            .ctx(&[Selection])
            .caps(&[C::ReadSelection, C::Clipboard])
            .perms(&sel_perm)
            .out(OutputBehavior::Copy)
            .d,
        builtin("clip.copy_plain", "Copy as Plain Text", "clipboard-type", Builtin::CopyPlainText)
            .ctx(&[Selection])
            .caps(&[C::ReadSelection, C::Clipboard])
            .perms(&sel_perm)
            .out(OutputBehavior::Copy)
            .d,
        builtin("snippet.save", "Save as Snippet", "bookmark-plus", Builtin::SaveAsSnippet)
            .ctx(&[Selection])
            .caps(&[C::ReadSelection])
            .perms(&sel_perm)
            .out(OutputBehavior::OpenSurface)
            .d,
        // ---- Editable field ----
        builtin("voice.start", "Start Voice Typing", "mic", Builtin::StartVoiceTyping)
            .ctx(&[Editable, Anywhere])
            .caps(&[C::Microphone, C::LocalTranscription, C::InsertText])
            .perms(&[Permission::Microphone, Permission::Accessibility])
            .out(OutputBehavior::OpenSurface)
            .d,
        builtin("ai.write", "Write with ChatGPT", "pen-line", Builtin::WriteWithAi)
            .ctx(&[Editable])
            .caps(&[C::AiConnection, C::InsertText])
            .perms(&sel_perm)
            .out(OutputBehavior::OpenSurface)
            .ai()
            .desc("Generate new text from a prompt and insert it after review.")
            .d,
        builtin("snippet.insert", "Insert Saved Snippet", "text-cursor-input", Builtin::InsertSnippet)
            .ctx(&[Editable])
            .caps(&[C::InsertText])
            .perms(&sel_perm)
            .out(OutputBehavior::OpenSurface)
            .d,
        builtin("clip.history", "Clipboard History", "clipboard-list", Builtin::ClipboardHistory)
            .ctx(&[Editable, Anywhere])
            .caps(&[C::ClipboardHistory])
            .out(OutputBehavior::OpenSurface)
            .d,
        builtin("clip.paste_plain", "Paste as Plain Text", "clipboard-paste", Builtin::PastePlainText)
            .ctx(&[Editable])
            .caps(&[C::Clipboard, C::InsertText])
            .perms(&sel_perm)
            .sec(SecurityClass::InputSynthesis)
            .d,
        builtin("insert.date", "Insert Current Date", "calendar", Builtin::InsertDate)
            .ctx(&[Editable])
            .caps(&[C::InsertText])
            .perms(&sel_perm)
            .sec(SecurityClass::InputSynthesis)
            .d,
        builtin("insert.time", "Insert Current Time", "clock", Builtin::InsertTime)
            .ctx(&[Editable])
            .caps(&[C::InsertText])
            .perms(&sel_perm)
            .sec(SecurityClass::InputSynthesis)
            .d,
        builtin("insert.uuid", "Insert UUID", "fingerprint-pattern", Builtin::UuidInsert)
            .group("dev")
            .ctx(&[Editable])
            .caps(&[C::InsertText])
            .perms(&sel_perm)
            .sec(SecurityClass::InputSynthesis)
            .d,
        // ---- Anywhere ----
        builtin("launcher.open", "Quick Launcher", "command", Builtin::QuickLauncher)
            .ctx(&[Anywhere])
            .out(OutputBehavior::OpenSurface)
            .d,
        builtin("ocr.capture", "Screen Text Capture", "scan-text", Builtin::ScreenTextCapture)
            .ctx(&[Anywhere])
            .caps(&[C::ScreenCapture, C::Ocr])
            .perms(&[Permission::ScreenRecording])
            .out(OutputBehavior::OpenSurface)
            .sec(SecurityClass::ScreenRead)
            .desc("Select a screen region and extract its text on-device.")
            .d,
        builtin("screenshot.area", "Take Screenshot", "camera", Builtin::Screenshot)
            .ctx(&[Anywhere])
            .caps(&[C::ScreenCapture])
            .perms(&[Permission::ScreenRecording])
            .sec(SecurityClass::ScreenRead)
            .d,
        win("window.left", "Left Half", "panel-left", Builtin::WindowLeftHalf),
        win("window.right", "Right Half", "panel-right", Builtin::WindowRightHalf),
        win("window.maximize", "Maximize", "maximize", Builtin::WindowMaximize),
        win("window.center", "Center", "square", Builtin::WindowCenter),
        win("window.top_left", "Top Left", "arrow-up-left", Builtin::WindowTopLeft),
        win("window.top_right", "Top Right", "arrow-up-right", Builtin::WindowTopRight),
        win("window.bottom_left", "Bottom Left", "arrow-down-left", Builtin::WindowBottomLeft),
        win("window.bottom_right", "Bottom Right", "arrow-down-right", Builtin::WindowBottomRight),
        win("window.next_display", "Move to Next Display", "monitor", Builtin::WindowNextDisplay),
        builtin("tts.stop", "Stop Speaking", "volume-x", Builtin::StopSpeaking).ctx(&[]).caps(&[C::Tts]).d,
        builtin("app.pause", "Pause MinPoint", "pause", Builtin::PauseMinPoint).ctx(&[]).d,
    ]
}

/// Display names for submenu groups.
pub fn group_label(g: &str) -> &'static str {
    match g {
        "tone" => "Change Tone",
        "case" => "Change Text Case",
        "convert" => "Convert To",
        "text" => "Text Utilities",
        "dev" => "Developer Tools",
        "window" => "Window Management",
        _ => "More",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_well_formed() {
        let acts = builtin_actions();
        let mut ids: Vec<_> = acts.iter().map(|a| a.id.clone()).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate action ids");
        for a in &acts {
            assert!(a.id.contains('.'), "{} should be namespaced", a.id);
            assert_eq!(a.requires_ai, a.capabilities.contains(&Capability::AiConnection), "{}", a.id);
            if a.requires_ai {
                assert!(!a.offline);
                assert_eq!(a.security, SecurityClass::ExternalAi);
            }
        }
    }
}
