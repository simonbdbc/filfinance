//! Full payload diagnostics are emitted only through calendar_logs when scoped by a calendar operation.
//! Closed official feeds. Documents are transient and never logged or persisted.
use std::collections::BTreeMap;

use quick_xml::{Reader, events::Event};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::{
    Date, OffsetDateTime, PrimitiveDateTime, UtcOffset,
    format_description::{
        self,
        well_known::{Rfc2822, Rfc3339},
    },
};

use super::collectors::CalendarLocalDay;
use crate::{
    features::calendar_day::contracts::{
        CalendarImportance, CalendarIngestionDraft, CalendarSourceKind,
    },
    shared::time::{europe_paris_day_for_utc, europe_paris_offset_for_utc},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarFeedSource {
    ForexFactoryExport,
    BeaJson,
    BeaIcs,
    FedFomc,
    FedMonetaryRss,
    FedSpeechesRss,
    EurostatIcs,
    EcbCalendar,
}

impl CalendarFeedSource {
    pub(crate) const ALL: [Self; 8] = [
        Self::ForexFactoryExport,
        Self::BeaJson,
        Self::BeaIcs,
        Self::FedFomc,
        Self::FedMonetaryRss,
        Self::FedSpeechesRss,
        Self::EurostatIcs,
        Self::EcbCalendar,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::ForexFactoryExport => "forex_factory_export",
            Self::BeaJson => "bea_json",
            Self::BeaIcs => "bea_ics",
            Self::FedFomc => "fed_fomc",
            Self::FedMonetaryRss => "fed_monetary_rss",
            Self::FedSpeechesRss => "fed_speeches_rss",
            Self::EurostatIcs => "eurostat_ics",
            Self::EcbCalendar => "ecb_calendar",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|source| source.name() == name)
    }

    pub(crate) fn endpoint(self) -> &'static str {
        match self {
            Self::ForexFactoryExport => "https://nfs.faireconomy.media/ff_calendar_thisweek.json",
            Self::BeaJson => "https://apps.bea.gov/API/signup/release_dates.json",
            Self::BeaIcs => {
                "https://www.bea.gov/news/schedule/ics/online-calendar-subscription.ics"
            }
            Self::FedFomc => "https://www.federalreserve.gov/monetarypolicy/fomccalendars.htm",
            Self::FedMonetaryRss => "https://www.federalreserve.gov/feeds/press_monetary.xml",
            Self::FedSpeechesRss => "https://www.federalreserve.gov/feeds/speeches.xml",
            // Official subscription button, all themes / Euro indicators releases.
            Self::EurostatIcs => {
                "https://ec.europa.eu/eurostat/o/calendars/eventsIcal?theme=0&category=2"
            }
            Self::EcbCalendar => {
                "https://www.ecb.europa.eu/press/calendars/statscal/html/index.en.html"
            }
        }
    }

    pub(crate) fn is_context(self) -> bool {
        matches!(self, Self::FedMonetaryRss | Self::FedSpeechesRss)
    }
}

// No Debug: the titles and source values must not enter diagnostics.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalendarFeedRecord {
    pub(crate) external_id: String,
    pub(crate) title: String,
    pub(crate) local_day: String,
    pub(crate) event_time: Option<String>,
    published_at: Option<String>,
    currency: Option<String>,
    source_importance: Option<CalendarImportance>,
    forecast: Option<String>,
    previous: Option<String>,
    // Preserve uncertain source time as context, never as an alert timestamp.
    time_note: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalendarFeedDocument {
    source: CalendarFeedSource,
    local_day: String,
    context_only: bool,
    pub(crate) records: Vec<CalendarFeedRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FeedParseError {
    InvalidFormat,
    LimitExceeded,
}

pub(crate) fn parse(
    source: CalendarFeedSource,
    raw: &str,
    day: CalendarLocalDay,
) -> Result<CalendarFeedDocument, FeedParseError> {
    let target = day.to_string();
    let mut records = match source {
        CalendarFeedSource::ForexFactoryExport => forex(raw)?,
        CalendarFeedSource::BeaJson => bea(raw)?,
        CalendarFeedSource::BeaIcs | CalendarFeedSource::EurostatIcs => ics(raw, source)?,
        CalendarFeedSource::FedFomc => fomc(raw, day)?,
        CalendarFeedSource::FedMonetaryRss | CalendarFeedSource::FedSpeechesRss => rss(raw)?,
        CalendarFeedSource::EcbCalendar => ecb(raw)?,
    };
    records.retain(|record| record.local_day == target);
    records.sort_by(|a, b| a.external_id.cmp(&b.external_id));
    records.dedup_by(|a, b| a.external_id == b.external_id);
    // Fail explicitly rather than silently omit releases or truncate JSON.
    if records.len() > 128 {
        return Err(FeedParseError::LimitExceeded);
    }
    let document = CalendarFeedDocument {
        source,
        local_day: target,
        context_only: source.is_context(),
        records,
    };
    if serde_json::to_vec(&document)
        .map_err(|_| FeedParseError::InvalidFormat)?
        .len()
        > 48_000
    {
        return Err(FeedParseError::LimitExceeded);
    }
    Ok(document)
}

fn record(
    title: &str,
    date: &str,
    instant: Option<OffsetDateTime>,
    currency: Option<&str>,
) -> Result<CalendarFeedRecord, FeedParseError> {
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() || title.chars().count() > 240 || title.contains(['<', '>']) {
        return Err(FeedParseError::InvalidFormat);
    }
    let event_time = instant
        .map(|v| v.to_offset(UtcOffset::UTC).format(&Rfc3339))
        .transpose()
        .map_err(|_| FeedParseError::InvalidFormat)?;
    let local_day = if let Some(v) = instant {
        europe_paris_day_for_utc(v)
            .ok_or(FeedParseError::InvalidFormat)?
            .to_string()
    } else {
        CalendarLocalDay::parse(date)
            .map_err(|_| FeedParseError::InvalidFormat)?
            .to_string()
    };
    let external_id = format!(
        "{:x}",
        Sha256::digest(
            format!(
                "{title}\n{local_day}\n{}",
                event_time.as_deref().unwrap_or_default()
            )
            .as_bytes()
        )
    );
    Ok(CalendarFeedRecord {
        external_id,
        title,
        local_day,
        event_time,
        published_at: None,
        currency: currency.map(str::to_owned),
        source_importance: None,
        forecast: None,
        previous: None,
        time_note: None,
    })
}

#[derive(Deserialize)]
struct ForexEvent {
    title: String,
    country: String,
    date: String,
    impact: String,
    #[serde(default)]
    forecast: String,
    #[serde(default)]
    previous: String,
}

fn forex(raw: &str) -> Result<Vec<CalendarFeedRecord>, FeedParseError> {
    let events: Vec<ForexEvent> =
        serde_json::from_str(raw).map_err(|_| FeedParseError::InvalidFormat)?;
    if events.is_empty() {
        return Err(FeedParseError::InvalidFormat);
    }
    events
        .into_iter()
        .map(|e| {
            let instant = OffsetDateTime::parse(&e.date, &Rfc3339)
                .map_err(|_| FeedParseError::InvalidFormat)?;
            let mut r = record(&e.title, "", Some(instant), Some(&e.country))?;
            r.source_importance = Some(match e.impact.as_str() {
                "High" => CalendarImportance::High,
                "Medium" => CalendarImportance::Medium,
                "Low" | "Holiday" => CalendarImportance::Low,
                _ => return Err(FeedParseError::InvalidFormat),
            });
            if e.impact == "Holiday" {
                r.event_time = None;
            }
            if e.forecast.len() > 160 || e.previous.len() > 160 {
                return Err(FeedParseError::LimitExceeded);
            }
            r.forecast = (!e.forecast.is_empty()).then_some(e.forecast);
            r.previous = (!e.previous.is_empty()).then_some(e.previous);
            Ok(r)
        })
        .collect()
}

#[derive(Deserialize)]
struct BeaRelease {
    release_dates: Vec<String>,
}
#[derive(Deserialize)]
struct BeaCalendar {
    #[serde(default, rename = "file_last_updated")]
    _file_last_updated: Option<String>,
    #[serde(flatten)]
    releases: BTreeMap<String, BeaRelease>,
}
fn bea(raw: &str) -> Result<Vec<CalendarFeedRecord>, FeedParseError> {
    let calendar: BeaCalendar =
        serde_json::from_str(raw).map_err(|_| FeedParseError::InvalidFormat)?;
    let releases = calendar.releases;
    if releases.is_empty() {
        return Err(FeedParseError::InvalidFormat);
    }
    let mut records = Vec::new();
    for (title, release) in releases {
        for date in release.release_dates {
            let instant = OffsetDateTime::parse(&date, &Rfc3339)
                .map_err(|_| FeedParseError::InvalidFormat)?;
            records.push(record(&title, "", Some(instant), Some("USD"))?);
        }
    }
    Ok(records)
}

// Restricted RFC5545 reader: no recurrence expansion, remote TZURL, attachments or URL fetches.
fn ics(raw: &str, source: CalendarFeedSource) -> Result<Vec<CalendarFeedRecord>, FeedParseError> {
    if !raw.starts_with("BEGIN:VCALENDAR") || !raw.trim_end().ends_with("END:VCALENDAR") {
        return Err(FeedParseError::InvalidFormat);
    }
    let mut lines: Vec<String> = Vec::new();
    for line in raw.lines() {
        if line.starts_with([' ', '\t']) {
            lines
                .last_mut()
                .ok_or(FeedParseError::InvalidFormat)?
                .push_str(&line[1..]);
        } else {
            lines.push(line.to_owned());
        }
    }
    let mut current: Option<BTreeMap<String, (String, String)>> = None;
    let mut records = Vec::new();
    for line in lines {
        if line == "BEGIN:VEVENT" {
            if current.is_some() {
                return Err(FeedParseError::InvalidFormat);
            }
            current = Some(BTreeMap::new());
        } else if line == "END:VEVENT" {
            let fields = current.take().ok_or(FeedParseError::InvalidFormat)?;
            if fields.get("STATUS").is_some_and(|(_, v)| v == "CANCELLED") {
                continue;
            }
            if fields.contains_key("RRULE") || fields.contains_key("RECURRENCE-ID") {
                return Err(FeedParseError::InvalidFormat);
            }
            let (params, date) = fields.get("DTSTART").ok_or(FeedParseError::InvalidFormat)?;
            let (_, title) = fields.get("SUMMARY").ok_or(FeedParseError::InvalidFormat)?;
            let title = title
                .replace("\\n", " ")
                .replace("\\N", " ")
                .replace("\\,", ",")
                .replace("\\;", ";")
                .replace("\\\\", "\\");
            let date_format = format_description::parse("[year][month][day]")
                .map_err(|_| FeedParseError::InvalidFormat)?;
            let (local, instant) = if params == ";VALUE=DATE" {
                (
                    Date::parse(date, &date_format)
                        .map_err(|_| FeedParseError::InvalidFormat)?
                        .to_string(),
                    None,
                )
            } else {
                let format = format_description::parse("[year][month][day]T[hour][minute][second]")
                    .map_err(|_| FeedParseError::InvalidFormat)?;
                let naive = PrimitiveDateTime::parse(date.trim_end_matches('Z'), &format)
                    .map_err(|_| FeedParseError::InvalidFormat)?;
                let instant =
                    if date.ends_with('Z') && matches!(params.as_str(), "" | ";VALUE=DATE-TIME") {
                        naive.assume_utc()
                    } else if params == ";TZID=Europe/Luxembourg" {
                        european_instant(naive)?
                    } else {
                        return Err(FeedParseError::InvalidFormat);
                    };
                (String::new(), Some(instant))
            };
            records.push(record(
                &title,
                &local,
                instant,
                Some(if source == CalendarFeedSource::BeaIcs {
                    "USD"
                } else {
                    "EUR"
                }),
            )?);
        } else if let Some(fields) = &mut current {
            let (key, value) = line.split_once(':').ok_or(FeedParseError::InvalidFormat)?;
            let (name, params) = key
                .split_once(';')
                .map(|(n, p)| (n, format!(";{p}")))
                .unwrap_or((key, String::new()));
            if matches!(
                name,
                "DTSTART" | "SUMMARY" | "STATUS" | "RRULE" | "RECURRENCE-ID"
            ) && fields
                .insert(name.to_owned(), (params, value.to_owned()))
                .is_some()
            {
                return Err(FeedParseError::InvalidFormat);
            }
        }
    }
    if current.is_some() {
        return Err(FeedParseError::InvalidFormat);
    }
    Ok(records)
}

fn european_instant(naive: PrimitiveDateTime) -> Result<OffsetDateTime, FeedParseError> {
    let candidates = [1, 2]
        .into_iter()
        .filter_map(|hour| {
            let offset = UtcOffset::from_hms(hour, 0, 0).ok()?;
            let instant = naive.assume_offset(offset);
            (europe_paris_offset_for_utc(instant) == Some(offset)).then_some(instant)
        })
        .collect::<Vec<_>>();
    // Reject gaps and ambiguous instants instead of choosing an alert time.
    if candidates.len() == 1 {
        Ok(candidates[0])
    } else {
        Err(FeedParseError::InvalidFormat)
    }
}

fn selector(value: &str) -> Selector {
    Selector::parse(value).expect("static feed selector")
}
fn text(element: scraper::ElementRef<'_>) -> String {
    element
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn fomc(raw: &str, day: CalendarLocalDay) -> Result<Vec<CalendarFeedRecord>, FeedParseError> {
    let html = Html::parse_document(raw);
    let day_text = day.to_string();
    let year = &day_text[..4];
    let panel = html
        .select(&selector(".panel"))
        .find(|panel| {
            panel
                .select(&selector(".panel-heading"))
                .any(|h| text(h) == format!("{year} FOMC Meetings"))
        })
        .ok_or(FeedParseError::InvalidFormat)?;
    let mut records = Vec::new();
    for meeting in panel.select(&selector(".fomc-meeting")) {
        let month = meeting
            .select(&selector(".fomc-meeting__month"))
            .next()
            .ok_or(FeedParseError::InvalidFormat)?;
        let month_text = text(month);
        let months = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        let month = months
            .iter()
            .position(|m| *m == month_text)
            .ok_or(FeedParseError::InvalidFormat)?
            + 1;
        let dates = text(
            meeting
                .select(&selector(".fomc-meeting__date"))
                .next()
                .ok_or(FeedParseError::InvalidFormat)?,
        );
        let dates = dates.trim_end_matches('*');
        let dates = dates
            .split('-')
            .map(str::parse::<u8>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| FeedParseError::InvalidFormat)?;
        if dates.is_empty() || dates.len() > 2 {
            return Err(FeedParseError::InvalidFormat);
        }
        let start = dates[0];
        let end = *dates.last().ok_or(FeedParseError::InvalidFormat)?;
        if end < start || end - start > 3 {
            return Err(FeedParseError::InvalidFormat);
        }
        for date in start..=end {
            let mut r = record(
                "FOMC meeting",
                &format!("{year}-{month:02}-{date:02}"),
                None,
                Some("USD"),
            )?;
            r.time_note =
                Some("Meeting day; decision time not specified by this calendar".to_owned());
            records.push(r);
        }
    }
    Ok(records)
}

fn ecb(raw: &str) -> Result<Vec<CalendarFeedRecord>, FeedParseError> {
    let html = Html::parse_document(raw);
    let mut records = Vec::new();
    for date in html.select(&selector(".definition-list dl dt")) {
        let date_text = text(date);
        let date_value = date_text
            .split_whitespace()
            .next()
            .ok_or(FeedParseError::InvalidFormat)?;
        let fmt = format_description::parse("[day]/[month]/[year]")
            .map_err(|_| FeedParseError::InvalidFormat)?;
        let day = Date::parse(date_value, &fmt).map_err(|_| FeedParseError::InvalidFormat)?;
        let description = date
            .next_siblings()
            .filter_map(scraper::ElementRef::wrap)
            .next()
            .filter(|el| el.value().name() == "dd")
            .ok_or(FeedParseError::InvalidFormat)?;
        let title = description.text().next().unwrap_or_default().trim();
        let mut r = record(title, &day.to_string(), None, Some("EUR"))?;
        // The HTML labels CET even during summer. Keep the evidence, not an assumed offset.
        r.time_note = Some(date_text);
        records.push(r);
    }
    if records.is_empty() {
        return Err(FeedParseError::InvalidFormat);
    }
    Ok(records)
}

fn rss(raw: &str) -> Result<Vec<CalendarFeedRecord>, FeedParseError> {
    let mut reader = Reader::from_str(raw);
    let mut stack: Vec<String> = Vec::new();
    let mut fields: Option<BTreeMap<String, String>> = None;
    let mut records = Vec::new();
    let mut saw_rss = false;
    loop {
        match reader
            .read_event()
            .map_err(|_| FeedParseError::InvalidFormat)?
        {
            Event::Start(e) => {
                let name = String::from_utf8(e.name().as_ref().to_vec())
                    .map_err(|_| FeedParseError::InvalidFormat)?;
                if stack.is_empty() {
                    saw_rss = name == "rss";
                }
                if name == "item" {
                    if fields.is_some() {
                        return Err(FeedParseError::InvalidFormat);
                    }
                    fields = Some(BTreeMap::new());
                }
                stack.push(name);
                if stack.len() > 32 {
                    return Err(FeedParseError::LimitExceeded);
                }
            }
            Event::Text(e) => {
                if let (Some(fields), Some(key)) = (&mut fields, stack.last()) {
                    if matches!(key.as_str(), "title" | "pubDate") {
                        fields
                            .entry(key.clone())
                            .or_default()
                            .push_str(&e.decode().map_err(|_| FeedParseError::InvalidFormat)?);
                    }
                }
            }
            Event::CData(e) => {
                if let (Some(fields), Some(key)) = (&mut fields, stack.last()) {
                    if matches!(key.as_str(), "title" | "pubDate") {
                        fields
                            .entry(key.clone())
                            .or_default()
                            .push_str(&e.decode().map_err(|_| FeedParseError::InvalidFormat)?);
                    }
                }
            }
            Event::GeneralRef(e) => {
                if let (Some(fields), Some(key)) = (&mut fields, stack.last()) {
                    if key == "title" {
                        let entity = format!(
                            "&{};",
                            e.decode().map_err(|_| FeedParseError::InvalidFormat)?
                        );
                        fields.entry(key.clone()).or_default().push_str(
                            &quick_xml::escape::unescape(&entity)
                                .map_err(|_| FeedParseError::InvalidFormat)?,
                        );
                    }
                }
            }
            Event::End(e) => {
                if e.name().as_ref() == b"item" {
                    let fields = fields.take().ok_or(FeedParseError::InvalidFormat)?;
                    let date = fields.get("pubDate").ok_or(FeedParseError::InvalidFormat)?;
                    let instant = OffsetDateTime::parse(date.trim(), &Rfc2822)
                        .map_err(|_| FeedParseError::InvalidFormat)?;
                    let mut r = record(
                        fields.get("title").ok_or(FeedParseError::InvalidFormat)?,
                        "",
                        Some(instant),
                        Some("USD"),
                    )?;
                    r.published_at = r.event_time.take();
                    records.push(r);
                }
                stack.pop().ok_or(FeedParseError::InvalidFormat)?;
            }
            Event::DocType(_) => return Err(FeedParseError::InvalidFormat),
            Event::Eof => break,
            _ => {}
        }
    }
    if !saw_rss || !stack.is_empty() || fields.is_some() {
        return Err(FeedParseError::InvalidFormat);
    }
    Ok(records)
}

/// An LLM cannot turn an RSS pubDate into a scheduled event or invent a timestamp.
/// Each new-feed provenance must reference the exact in-memory record supplied in the prompt.
pub(crate) fn validate_output(
    draft: &CalendarIngestionDraft,
    documents: &[&CalendarFeedDocument],
) -> bool {
    let preserves_high = documents.iter().all(|document| {
        document
            .records
            .iter()
            .filter(|record| record.source_importance == Some(CalendarImportance::High))
            .all(|record| {
                draft.items.iter().any(|item| {
                    item.sources.iter().any(|provenance| {
                        provenance.source == CalendarSourceKind::Feed(document.source)
                            && provenance.external_id.as_deref() == Some(&record.external_id)
                    })
                })
            })
    });
    preserves_high
        && draft.items.iter().all(|item| {
            let mut scheduled = Vec::new();
            let mut has_feed = false;
            for provenance in &item.sources {
                let CalendarSourceKind::Feed(source) = provenance.source;
                has_feed = true;
                let Some(document) = documents.iter().find(|d| d.source == source) else {
                    return false;
                };
                let Some(record) = document.records.iter().find(|r| {
                    Some(r.external_id.as_str()) == provenance.external_id.as_deref()
                        && Some(r.title.as_str()) == provenance.raw_title.as_deref()
                }) else {
                    return false;
                };
                if !document.context_only {
                    scheduled.push(record);
                }
            }
            if !has_feed {
                return documents.is_empty();
            } // Historical fixtures only, never production.
            if scheduled.is_empty() {
                return false;
            }
            if scheduled
                .iter()
                .any(|r| r.source_importance == Some(CalendarImportance::High))
                && item.final_importance != CalendarImportance::High
            {
                return false;
            }
            if item.event_time.is_none() && scheduled.iter().any(|r| r.event_time.is_some()) {
                return false;
            }
            if let Some(time) = &item.event_time {
                let Ok(instant) = OffsetDateTime::parse(time, &Rfc3339) else {
                    return false;
                };
                if !scheduled.iter().any(|r| {
                    r.event_time
                        .as_deref()
                        .and_then(|v| OffsetDateTime::parse(v, &Rfc3339).ok())
                        == Some(instant)
                }) {
                    return false;
                }
            }
            true
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::calendar_day::{
        contracts::{self, CalendarSourceSnapshot},
        native::prompts::build_ingestion_prompt,
    };
    use serde_json::json;

    fn day() -> CalendarLocalDay {
        "2026-09-07".parse().unwrap()
    }
    fn ff() -> String {
        json!([
        {"title":"Test GDP","country":"USD","date":"2026-09-06T20:30:00-04:00","impact":"High","forecast":"2%","previous":"1%"},
        {"title":"Other day","country":"EUR","date":"2026-09-08T10:00:00+02:00","impact":"Low"}
    ]).to_string()
    }
    fn calendar(start: &str, title: &str, extra: &str) -> String {
        format!(
            "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nDTSTART{start}\r\nSUMMARY:{title}\r\n{extra}END:VEVENT\r\nEND:VCALENDAR\r\n"
        )
    }

    #[test]
    fn export_filters_exact_paris_day_with_offset_and_preserves_values() {
        let d = parse(CalendarFeedSource::ForexFactoryExport, &ff(), day()).unwrap();
        assert_eq!(d.records.len(), 1);
        assert_eq!(
            d.records[0].event_time.as_deref(),
            Some("2026-09-07T00:30:00Z")
        );
        assert_eq!(d.records[0].currency.as_deref(), Some("USD"));
        assert_eq!(d.records[0].forecast.as_deref(), Some("2%"));
        assert_eq!(
            d.records[0].source_importance,
            Some(CalendarImportance::High)
        );
        assert!(
            parse(
                CalendarFeedSource::ForexFactoryExport,
                &ff(),
                "2027-09-07".parse().unwrap()
            )
            .unwrap()
            .records
            .is_empty()
        );
        assert!(
            parse(
                CalendarFeedSource::ForexFactoryExport,
                "<html>2026-09-07</html>",
                day()
            )
            .is_err()
        );
    }

    #[test]
    fn feed_limits_fail_closed_and_high_records_cannot_disappear() {
        let rows=(0..129).map(|i| json!({"title":format!("Release {i}"),"country":"USD","date":"2026-09-07T12:00:00Z","impact":"High"})).collect::<Vec<_>>();
        assert!(matches!(
            parse(
                CalendarFeedSource::ForexFactoryExport,
                &serde_json::to_string(&rows).unwrap(),
                day()
            ),
            Err(FeedParseError::LimitExceeded)
        ));
        let d = parse(
            CalendarFeedSource::ForexFactoryExport,
            &serde_json::to_string(&rows[..2]).unwrap(),
            day(),
        )
        .unwrap();
        let one = draft(&d);
        assert!(
            !validate_output(&one, &[&d]),
            "a high source event must not silently disappear"
        );
    }

    #[test]
    fn bea_json_and_ics_share_stable_identity_and_unfold_escaped_titles() {
        let json = r#"{"file_last_updated":"2026-07-13T08:00:42.402013","Test GDP, quarterly":{"release_dates":["2026-09-07T12:30:00Z","2026-09-07T12:30:00Z"]}}"#;
        let a = parse(CalendarFeedSource::BeaJson, json, day()).unwrap();
        let b = parse(
            CalendarFeedSource::BeaIcs,
            &calendar(
                ";VALUE=DATE-TIME:20260907T123000Z",
                "Test GDP\\, quar\r\n terly",
                "",
            ),
            day(),
        )
        .unwrap();
        assert_eq!(a.records.len(), 1);
        assert_eq!(a.records[0].external_id, b.records[0].external_id);
        assert_eq!(a.records[0].event_time, b.records[0].event_time);
    }

    #[test]
    fn ics_all_day_is_not_midnight_and_cancelled_releases_are_excluded() {
        let raw = calendar(";VALUE=DATE:20260907", "Inflation", "");
        let d = parse(CalendarFeedSource::EurostatIcs, &raw, day()).unwrap();
        assert_eq!(d.records.len(), 1);
        assert!(d.records[0].event_time.is_none());
        let cancelled = calendar(";VALUE=DATE:20260907", "Inflation", "STATUS:CANCELLED\r\n");
        assert!(
            parse(CalendarFeedSource::EurostatIcs, &cancelled, day())
                .unwrap()
                .records
                .is_empty()
        );
        for extra in [
            "RRULE:FREQ=DAILY\r\n",
            "RECURRENCE-ID:20260907T120000Z\r\n",
            "DTSTART:20260907T150000Z\r\n",
        ] {
            assert!(
                parse(
                    CalendarFeedSource::EurostatIcs,
                    &calendar(";VALUE=DATE:20260907", "Test", extra),
                    day()
                )
                .is_err()
            );
        }
    }

    #[test]
    fn ics_timezone_is_explicit_and_dst_gaps_and_duplicates_are_rejected() {
        let d = parse(
            CalendarFeedSource::EurostatIcs,
            &calendar(";TZID=Europe/Luxembourg:20260907T110000", "GDP", ""),
            day(),
        )
        .unwrap();
        assert_eq!(
            d.records[0].event_time.as_deref(),
            Some("2026-09-07T09:00:00Z")
        );
        for start in [
            ":20260907T110000",
            ";TZID=Unknown:20260907T110000",
            ";TZID=Europe/Luxembourg:20260329T023000",
            ";TZID=Europe/Luxembourg:20261025T023000",
        ] {
            assert!(
                parse(
                    CalendarFeedSource::EurostatIcs,
                    &calendar(start, "Test", ""),
                    day()
                )
                .is_err()
            );
        }
    }

    #[test]
    fn fomc_requires_matching_year_and_covers_meeting_range_without_an_hour() {
        let raw = "<div class='panel'><div class='panel-heading'>2026 FOMC Meetings</div><div class='fomc-meeting'><div class='fomc-meeting__month'>September</div><div class='fomc-meeting__date'>7-8*</div></div></div>";
        let d = parse(CalendarFeedSource::FedFomc, raw, day()).unwrap();
        assert_eq!(d.records.len(), 1);
        assert!(d.records[0].event_time.is_none());
        assert!(
            parse(
                CalendarFeedSource::FedFomc,
                raw,
                "2027-09-07".parse().unwrap()
            )
            .is_err()
        );
        assert!(
            parse(
                CalendarFeedSource::FedFomc,
                raw,
                "2026-09-09".parse().unwrap()
            )
            .unwrap()
            .records
            .is_empty()
        );
    }

    #[test]
    fn ecb_tentative_or_ambiguous_cet_times_are_not_alert_instants() {
        let d=parse(CalendarFeedSource::EcbCalendar,"<div class='definition-list'><dl><dt>07/09/2026 10:00 CET<br>Tentative</dt><dd>Statistical release<br>Reference period: August</dd></dl></div>",day()).unwrap();
        assert_eq!(d.records.len(), 1);
        assert_eq!(d.records[0].title, "Statistical release");
        assert!(d.records[0].event_time.is_none());
        assert!(
            d.records[0]
                .time_note
                .as_ref()
                .unwrap()
                .contains("Tentative")
        );
    }

    fn rss_fixture() -> &'static str {
        "<rss><channel><item><title>Fed &amp; policy</title><pubDate>Mon, 07 Sep 2026 12:30:00 GMT</pubDate></item></channel></rss>"
    }
    #[test]
    fn rss_is_context_never_a_schedule_and_rejects_dtd_or_broken_xml() {
        let d = parse(CalendarFeedSource::FedMonetaryRss, rss_fixture(), day()).unwrap();
        assert!(d.context_only);
        assert_eq!(d.records[0].title, "Fed & policy");
        assert!(d.records[0].published_at.is_some());
        assert!(d.records[0].event_time.is_none());
        for raw in [
            "<!DOCTYPE rss [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><rss/>",
            "<rss><channel><item></rss>",
            "<html>2026-09-07</html>",
        ] {
            assert!(parse(CalendarFeedSource::FedMonetaryRss, raw, day()).is_err());
        }
    }

    fn snapshot(d: &CalendarFeedDocument) -> CalendarSourceSnapshot {
        CalendarSourceSnapshot::from_response_text(
            "2026-09-07",
            CalendarSourceKind::Feed(d.source),
            d.source.endpoint(),
            200,
            &serde_json::to_string(d).unwrap(),
        )
        .unwrap()
    }
    fn draft(d: &CalendarFeedDocument) -> CalendarIngestionDraft {
        let r = &d.records[0];
        // Scheduled snapshot is necessary even for the malicious RSS-only model fixture.
        let covered = parse(CalendarFeedSource::ForexFactoryExport, &ff(), day()).unwrap();
        contracts::canonicalize_ingestion_output("2026-09-07",&[snapshot(&covered)],&json!({"items":[{
            "eventTime":r.event_time,"sourceTitle":r.title,"translatedTitle":"Decision de taux de la BCE",
            "ttsText":"Evenement calendrier. Decision de taux de la BCE.","reasoning":"Decision majeure pour les marches europeens.",
            "sourceImportance":"high","aiImportance":"high","finalImportance":"high",
            "sources":[{"source":d.source.name(),"externalId":r.external_id,"rawTitle":r.title}]
        }]}).to_string()).unwrap()
    }

    #[test]
    fn model_cannot_invent_time_id_or_rss_only_event() {
        let d = parse(CalendarFeedSource::ForexFactoryExport, &ff(), day()).unwrap();
        let good = draft(&d);
        assert!(validate_output(&good, &[&d]));
        let mut missing_time = good.clone();
        missing_time.items[0].event_time = None;
        assert!(!validate_output(&missing_time, &[&d]));
        let mut downgraded = good.clone();
        downgraded.items[0].final_importance = contracts::CalendarImportance::Low;
        assert!(!validate_output(&downgraded, &[&d]));
        let mut bad = good.clone();
        bad.items[0].event_time = Some("2026-09-07T15:00:00Z".to_owned());
        assert!(!validate_output(&bad, &[&d]));
        let mut bad = good.clone();
        bad.items[0].sources[0].external_id = Some("invented".to_owned());
        assert!(!validate_output(&bad, &[&d]));
        let mut bad = good.clone();
        bad.items[0].sources[0].raw_title = Some("invented".to_owned());
        assert!(!validate_output(&bad, &[&d]));
        let context = parse(CalendarFeedSource::FedMonetaryRss, rss_fixture(), day()).unwrap();
        assert!(!snapshot(&context).target_day_found);
        assert!(!validate_output(&draft(&context), &[&context]));
    }

    #[test]
    fn every_new_source_roundtrips_and_prompt_keeps_parseable_documents() {
        for source in CalendarFeedSource::ALL {
            let kind = CalendarSourceKind::Feed(source);
            assert_eq!(
                serde_json::to_string(&kind).unwrap(),
                format!("\"{}\"", source.name())
            );
            assert_eq!(
                CalendarSourceKind::from_name(source.name()),
                Some(kind.clone())
            );
            assert!(
                contracts::CalendarCoveredSourceBinding::for_local_day(
                    "2026-09-07",
                    kind,
                    url::Url::parse(source.endpoint()).unwrap()
                )
                .is_ok()
            );
        }
        let scheduled = parse(CalendarFeedSource::ForexFactoryExport, &ff(), day()).unwrap();
        let context = parse(CalendarFeedSource::FedMonetaryRss, rss_fixture(), day()).unwrap();
        let prompt =
            build_ingestion_prompt("2026-09-07", &[snapshot(&scheduled), snapshot(&context)])
                .unwrap();
        assert!(prompt.content().contains("JAMAIS"));
        let payload: serde_json::Value =
            serde_json::from_str(prompt.content().lines().last().unwrap()).unwrap();
        assert_eq!(payload["webSources"].as_array().unwrap().len(), 2);
        for source in payload["webSources"].as_array().unwrap() {
            assert!(
                serde_json::from_str::<serde_json::Value>(source["text"].as_str().unwrap()).is_ok()
            );
        }
        assert!(!format!("{:?}", snapshot(&scheduled)).contains("Test GDP"));
    }
}
