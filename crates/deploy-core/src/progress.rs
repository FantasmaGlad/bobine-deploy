use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Kiosk,
    Server,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Start,
    Ok,
    Skip,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunBegin {
    #[serde(default)]
    pub version: String,
    pub total: u32,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub log: String,
    pub mode: Mode,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepEvent {
    pub status: StepStatus,
    pub step: u32,
    pub total: u32,
    pub pct: u8,
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunEnd {
    pub status: RunStatus,
    #[serde(default)]
    pub pct: Option<u8>,
    pub total: u32,
    #[serde(default)]
    pub healthy: Option<bool>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub rc: Option<i32>,
    #[serde(default)]
    pub step: Option<u32>,
    #[serde(default)]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    RunBegin(RunBegin),
    Step(StepEvent),
    RunEnd(RunEnd),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    Event(Event),
    Malformed(String),
    Log,
}

pub fn is_event_marker(line: &str) -> bool {
    line.trim_start().starts_with("{\"bobine\":")
}

pub fn classify_line(line: &str) -> LineKind {
    let trimmed = line.trim();
    if is_event_marker(trimmed) {
        match serde_json::from_str::<Event>(trimmed) {
            Ok(ev) => LineKind::Event(ev),
            Err(e) => LineKind::Malformed(e.to_string()),
        }
    } else {
        LineKind::Log
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    NotStarted,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepRef {
    pub index: u32,
    pub slug: String,
    pub title: String,
}

impl StepRef {
    fn from(ev: &StepEvent) -> Self {
        StepRef {
            index: ev.step,
            slug: ev.slug.clone(),
            title: ev.title.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressState {
    pub phase: Phase,
    pub total: u32,
    pub pct: u8,
    pub current: Option<StepRef>,
    pub completed: u32,
    pub failure: Option<StepRef>,
    pub run: Option<RunBegin>,
    pub end: Option<RunEnd>,
}

impl Default for ProgressState {
    fn default() -> Self {
        ProgressState {
            phase: Phase::NotStarted,
            total: 0,
            pct: 0,
            current: None,
            completed: 0,
            failure: None,
            run: None,
            end: None,
        }
    }
}

impl ProgressState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&mut self, ev: &Event) {
        match ev {
            Event::RunBegin(rb) => {
                self.total = rb.total;
                self.phase = Phase::Running;
                self.pct = 0;
                self.run = Some(rb.clone());
            }
            Event::Step(s) => {
                self.total = s.total;
                match s.status {
                    StepStatus::Start => {
                        if self.phase == Phase::NotStarted {
                            self.phase = Phase::Running;
                        }
                        self.pct = s.pct;
                        self.current = Some(StepRef::from(s));
                    }
                    StepStatus::Ok | StepStatus::Skip => {
                        self.completed += 1;
                        self.pct = s.pct;
                        self.current = Some(StepRef::from(s));
                    }
                    StepStatus::Error => {
                        self.pct = s.pct;
                        self.failure = Some(StepRef::from(s));
                        self.phase = Phase::Failed;
                    }
                }
            }
            Event::RunEnd(re) => {
                self.end = Some(re.clone());
                match re.status {
                    RunStatus::Ok => {
                        self.phase = Phase::Succeeded;
                        self.pct = re.pct.unwrap_or(100);
                    }
                    RunStatus::Error => {
                        self.phase = Phase::Failed;
                        if let Some(p) = re.pct {
                            self.pct = p;
                        }
                    }
                }
            }
        }
    }

    pub fn feed_line(&mut self, line: &str) -> LineKind {
        let kind = classify_line(line);
        if let LineKind::Event(ev) = &kind {
            self.apply(ev);
        }
        kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_run_begin_with_escaped_values() {
        let line = r#"{"bobine":1,"ts":"2026-08-14T10:00:00Z","event":"run_begin","version":"2.0","total":15,"user":"fa\"n\\ta","repo":"/home/fa nta/Bob\"ine","log":"/tmp/x.log","mode":"kiosk","dry_run":false}"#;
        match classify_line(line) {
            LineKind::Event(Event::RunBegin(rb)) => {
                assert_eq!(rb.total, 15);
                assert_eq!(rb.user, "fa\"n\\ta");
                assert_eq!(rb.repo, "/home/fa nta/Bob\"ine");
                assert_eq!(rb.mode, Mode::Kiosk);
                assert!(!rb.dry_run);
            }
            other => panic!("attendu RunBegin, obtenu {other:?}"),
        }
    }

    #[test]
    fn parses_step_skip_with_detail() {
        let line = r#"{"bobine":1,"ts":"t","event":"step","status":"skip","step":4,"total":15,"pct":26,"slug":"frontend-build","title":"Build du frontend Next.js","detail":"--skip-build"}"#;
        match classify_line(line) {
            LineKind::Event(Event::Step(s)) => {
                assert_eq!(s.status, StepStatus::Skip);
                assert_eq!(s.slug, "frontend-build");
                assert_eq!(s.detail.as_deref(), Some("--skip-build"));
            }
            other => panic!("attendu Step, obtenu {other:?}"),
        }
    }

    #[test]
    fn non_event_line_is_log() {
        assert_eq!(classify_line("  ┃ [1/15] Paquets système (apt)"), LineKind::Log);
        assert_eq!(classify_line("ok redis-server actif"), LineKind::Log);
        assert_eq!(classify_line(r#"{"autre":true}"#), LineKind::Log);
    }

    #[test]
    fn marked_but_broken_line_is_malformed() {
        match classify_line(r#"{"bobine":1,"event":"step","status":"#) {
            LineKind::Malformed(_) => {}
            other => panic!("attendu Malformed, obtenu {other:?}"),
        }
    }

    #[test]
    fn state_tracks_start_then_close() {
        let mut st = ProgressState::new();
        st.feed_line(r#"{"bobine":1,"event":"run_begin","total":15,"mode":"kiosk"}"#);
        assert_eq!(st.phase, Phase::Running);
        st.feed_line(r#"{"bobine":1,"event":"step","status":"start","step":1,"total":15,"pct":0,"slug":"packages","title":"Paquets"}"#);
        assert_eq!(st.pct, 0);
        assert_eq!(st.current.as_ref().unwrap().slug, "packages");
        st.feed_line(r#"{"bobine":1,"event":"step","status":"ok","step":1,"total":15,"pct":6,"slug":"packages","title":"Paquets"}"#);
        assert_eq!(st.pct, 6);
        assert_eq!(st.completed, 1);
    }

    #[test]
    fn state_marks_failure_on_error_events() {
        let mut st = ProgressState::new();
        st.feed_line(r#"{"bobine":1,"event":"step","status":"start","step":1,"total":15,"pct":0,"slug":"packages","title":"Paquets"}"#);
        st.feed_line(r#"{"bobine":1,"event":"step","status":"error","step":1,"total":15,"pct":0,"slug":"packages","title":"Paquets","detail":"code 100"}"#);
        assert_eq!(st.phase, Phase::Failed);
        assert_eq!(st.failure.as_ref().unwrap().slug, "packages");
        st.feed_line(r#"{"bobine":1,"event":"run_end","status":"error","rc":100,"step":1,"total":15}"#);
        assert_eq!(st.phase, Phase::Failed);
        assert_eq!(st.end.as_ref().unwrap().rc, Some(100));
    }

    #[test]
    fn run_end_ok_forces_hundred_percent() {
        let mut st = ProgressState::new();
        st.feed_line(r#"{"bobine":1,"event":"run_end","status":"ok","pct":100,"total":15,"healthy":true,"url":"http://bobine.local","port":1903,"dry_run":false}"#);
        assert_eq!(st.phase, Phase::Succeeded);
        assert_eq!(st.pct, 100);
        assert_eq!(st.end.as_ref().unwrap().healthy, Some(true));
    }
}
