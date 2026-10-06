//! log4net(`LogConfig.xml`) 설정과 같은 형식으로 콘솔과 날짜별 파일에 로그를 남긴다.
//!
//! - 콘솔: `%-5p %d{yyyy-MM-dd HH:mm:ss} : %m%n` (표준 출력)
//! - 파일: 실행 파일 디렉터리의 `logs/yyyy-MM-dd.log`,
//!   `%-5p %d{yyyy-MM-dd HH:mm:ss} [%t][%l] : %m%n`
//! - 수준: INFO 이상

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{Local, NaiveDate};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, MakeWriter};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// 로거를 전역으로 등록한다. 두 번째 호출부터는 아무 일도 하지 않는다.
pub fn init() {
    let console = tracing_subscriber::fmt::layer()
        .event_format(Log4netFormat { detail: false })
        .with_writer(io::stdout);
    let file = tracing_subscriber::fmt::layer()
        .event_format(Log4netFormat { detail: true })
        .with_ansi(false)
        .with_writer(DailyFile::new(log_dir()));
    let _ = tracing_subscriber::registry()
        .with(console.and_then(file).with_filter(LevelFilter::INFO))
        .try_init();
}

/// log4net은 상대 경로를 애플리케이션 기준 디렉터리(실행 파일 위치)에서 해석한다.
fn log_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("logs")))
        .unwrap_or_else(|| PathBuf::from("logs"))
}

fn level_name(level: &Level) -> &'static str {
    match *level {
        Level::TRACE => "TRACE",
        Level::DEBUG => "DEBUG",
        Level::INFO => "INFO",
        Level::WARN => "WARN",
        Level::ERROR => "ERROR",
    }
}

/// log4net `PatternLayout`과 같은 한 줄 형식.
struct Log4netFormat {
    /// 파일 로그처럼 `[%t][%l]`(스레드, 위치)를 붙일지 여부.
    detail: bool,
}

impl<S, N> FormatEvent<S, N> for Log4netFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let meta = event.metadata();
        write!(
            writer,
            "{:<5} {}",
            level_name(meta.level()),
            Local::now().format(TIME_FORMAT)
        )?;
        if self.detail {
            write!(
                writer,
                " [{}][{}({}:{})]",
                thread_label(),
                meta.target(),
                meta.file().unwrap_or("?"),
                meta.line().unwrap_or(0)
            )?;
        }
        write!(writer, " : ")?;
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

/// `%t`: 스레드 이름이 있으면 이름, 없으면 번호.
fn thread_label() -> String {
    let thread = std::thread::current();
    match thread.name() {
        Some(name) => name.to_string(),
        None => format!("{:?}", thread.id())
            .chars()
            .filter(char::is_ascii_digit)
            .collect(),
    }
}

/// log4net `RollingFileAppender`(`rollingStyle=Date`, `datePattern=yyyy-MM-dd'.log'`)처럼
/// 현지 날짜가 바뀌면 새 파일에 이어 쓴다. 파일을 열 수 없으면 조용히 버린다.
struct DailyFile {
    dir: PathBuf,
    current: Mutex<Option<(NaiveDate, File)>>,
}

impl DailyFile {
    fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            current: Mutex::new(None),
        }
    }

    fn open(&self, date: NaiveDate) -> io::Result<File> {
        fs::create_dir_all(&self.dir)?;
        let path = self.dir.join(format!("{}.log", date.format("%Y-%m-%d")));
        OpenOptions::new().create(true).append(true).open(path)
    }
}

impl Write for &DailyFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let today = Local::now().date_naive();
        let mut current = self.current.lock().unwrap_or_else(|e| e.into_inner());
        if current.as_ref().is_none_or(|(date, _)| *date != today) {
            *current = self.open(today).ok().map(|file| (today, file));
        }
        if let Some((_, file)) = current.as_mut() {
            let _ = file.write_all(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut current = self.current.lock().unwrap_or_else(|e| e.into_inner());
        match current.as_mut() {
            Some((_, file)) => file.flush(),
            None => Ok(()),
        }
    }
}

impl<'a> MakeWriter<'a> for DailyFile {
    type Writer = &'a DailyFile;

    fn make_writer(&'a self) -> Self::Writer {
        self
    }
}
