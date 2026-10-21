//! Acceptance orchestration only; no runtime profile or host preparation policy.
use gr_curated_controllers::registry::{FAMILIES, FamilyEntry};
use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Instant};

#[cfg(target_os = "linux")]
pub mod linux;

pub const STAGES: &[&str] = &[
    "api_call",
    "api_returned",
    "node_present",
    "consumer_open",
    "first_neutral",
    "sdl_ready",
    "close_call",
    "close_returned",
    "node_gone",
];

pub trait Clock {
    fn now_ns(&mut self) -> u64;
}
pub struct Monotonic(Instant);
impl Default for Monotonic {
    fn default() -> Self {
        Self(Instant::now())
    }
}
impl Clock for Monotonic {
    fn now_ns(&mut self) -> u64 {
        u64::try_from(self.0.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

/// Every observation is specific to one owned creation. Missing data is a reason.
pub trait Backend {
    type Session;
    /// # Errors
    /// Returns a provider creation failure.
    fn create(&mut self, family: &str, realization: &str) -> Result<Self::Session, String>;
    /// # Errors
    /// Returns why this stage could not be observed.
    fn observe(
        &mut self,
        session: &mut Self::Session,
        stage: &str,
        peers: &mut [&mut Self::Session],
    ) -> Result<Value, String>;
    /// # Errors
    /// Returns the first scripted input/service failure.
    fn drive(&mut self, session: &mut Self::Session) -> Result<(), String>;
    fn close(&mut self, session: &mut Self::Session);
    /// Drain service failures belonging to this creation, including failures
    /// observed while a different creation was being measured.
    fn take_service_errors(&mut self, _: &mut Self::Session) -> Vec<String> {
        Vec::new()
    }
    fn resources(&mut self) -> Value;
}

/// A failed peer is not serviced again while another sample is observed.
#[derive(Default)]
pub(crate) struct PeerService {
    failure: Option<String>,
}
impl PeerService {
    pub(crate) fn service(&mut self, operation: impl FnOnce() -> Result<(), String>) {
        if self.failure.is_none() {
            self.failure = operation().err();
        }
    }
    pub(crate) fn take_errors(&mut self) -> Vec<String> {
        self.failure.take().into_iter().collect()
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub controller: String,
    pub realization: String,
    pub cycles: usize,
    pub concurrency: usize,
}
impl Config {
    /// # Errors
    /// Rejects unknown or undeclared pairs and unbounded workloads before provider I/O.
    pub fn family(&self) -> Result<&'static FamilyEntry, String> {
        if !(1..=1000).contains(&self.cycles) || !(1..=32).contains(&self.concurrency) {
            return Err("cycles must be 1..1000 and concurrency 1..32".into());
        }
        FAMILIES
            .iter()
            .find(|f| {
                f.id.as_str() == self.controller
                    && (f.manifest)()
                        .entries()
                        .iter()
                        .any(|e| e.target.as_str() == self.realization)
            })
            .ok_or_else(|| "unknown controller or undeclared realization".into())
    }
}

fn measured(at: u64) -> Value {
    json!({"value_ns":at,"reason":null})
}
fn missing(reason: &str) -> Value {
    json!({"value_ns":null,"reason":reason})
}

/// Runs all creations in a cycle before closing any, preserving failures as samples.
/// # Errors
/// Invalid configuration is rejected before opening a provider.
pub fn run<B: Backend, C: Clock>(
    backend: &mut B,
    clock: &mut C,
    config: &Config,
    environment: &Value,
) -> Result<Value, String> {
    let family = config.family()?;
    let realization = (family.realizations)()
        .iter()
        .find(|r| r.surface.target().as_str() == config.realization)
        .ok_or("registry realization metadata missing")?;
    let baseline = backend.resources();
    let mut samples = Vec::new();
    for cycle in 0..config.cycles {
        let mut active = Vec::new();
        for index in 0..config.concurrency {
            let mut stages: BTreeMap<_, _> = STAGES
                .iter()
                .map(|s| ((*s).to_owned(), missing("not reached")))
                .collect();
            stages.insert("api_call".into(), measured(clock.now_ns()));
            let mut errors = Vec::new();
            match backend.create(&config.controller, &config.realization) {
                Ok(mut session) => {
                    stages.insert("api_returned".into(), measured(clock.now_ns()));
                    let mut observations = BTreeMap::new();
                    for stage in ["node_present", "consumer_open", "first_neutral", "sdl_ready"] {
                        match backend.observe(&mut session, stage, &mut active.iter_mut().map(|entry: &mut (B::Session, _, _, _, _)| &mut entry.0).collect::<Vec<_>>()) {
                            Ok(value) => { stages.insert(stage.into(), measured(clock.now_ns())); observations.insert(stage, value); }
                            Err(reason) => { stages.insert(stage.into(), missing(&reason)); }
                        }
                    }
                    if let Err(error) = backend.drive(&mut session) { errors.push(error); }
                    active.push((session, stages, observations, errors, index));
                }
                Err(error) => samples.push(json!({"cycle":cycle,"index":index,"temperature":if cycle==0 {"cold"}else{"warm"},"stages":stages,"observations":{},"errors":[error]})),
            }
        }
        while !active.is_empty() {
            let (mut session, mut stages, mut observations, mut errors, index) = active.remove(0);
            stages.insert("close_call".into(), measured(clock.now_ns()));
            backend.close(&mut session);
            stages.insert("close_returned".into(), measured(clock.now_ns()));
            backend.close(&mut session); // idempotence exercised on every creation
            match backend.observe(
                &mut session,
                "node_gone",
                &mut active
                    .iter_mut()
                    .map(|entry| &mut entry.0)
                    .collect::<Vec<_>>(),
            ) {
                Ok(value) => {
                    stages.insert("node_gone".into(), measured(clock.now_ns()));
                    observations.insert("node_gone", value);
                }
                Err(error) => {
                    stages.insert("node_gone".into(), missing(&error));
                    errors.push(error);
                }
            }
            errors.extend(backend.take_service_errors(&mut session));
            drop(session);
            samples.push(json!({"cycle":cycle,"index":index,"temperature":if cycle==0 {"cold"}else{"warm"},"stages":stages,"observations":observations,"errors":errors}));
        }
    }
    samples.sort_by_key(|s| (s["cycle"].as_u64(), s["index"].as_u64()));
    let after = backend.resources();
    let summaries = summaries(&samples);
    Ok(
        json!({"schema_version":1,"kind":"virtual_host_run","controller":config.controller,
        "realization":config.realization,"tier":format!("{:?}",realization.tier),"permission":format!("{:?}",realization.permission),
        "config":{"cycles":config.cycles,"concurrency":config.concurrency},"environment":environment,
        "resources":{"before":baseline,"after":after},"summaries":summaries,"samples":samples}),
    )
}

fn summaries(samples: &[Value]) -> Value {
    let mut result = BTreeMap::new();
    for temperature in ["cold", "warm"] {
        for stage in STAGES
            .iter()
            .filter(|s| !matches!(**s, "api_call" | "close_call"))
        {
            let relevant: Vec<_> = samples
                .iter()
                .filter(|s| s["temperature"] == temperature)
                .collect();
            let mut values: Vec<_> = relevant
                .iter()
                .filter_map(|s| {
                    let start = if matches!(*stage, "close_returned" | "node_gone") {
                        "close_call"
                    } else {
                        "api_call"
                    };
                    s["stages"][stage]["value_ns"]
                        .as_u64()?
                        .checked_sub(s["stages"][start]["value_ns"].as_u64()?)
                })
                .collect();
            values.sort_unstable();
            result.insert(
                format!("{temperature}/{stage}"),
                statistics(&values, relevant.len() - values.len()),
            );
        }
    }
    json!(result)
}

// Floating-point summaries are descriptive only; raw u64 nanoseconds remain authoritative.
#[allow(clippy::cast_precision_loss)]
fn statistics(values: &[u64], missing: usize) -> Value {
    if values.is_empty() {
        return json!({"n":0,"missing":missing});
    }
    let percentile = |p: usize| values[(values.len() * p).div_ceil(100).saturating_sub(1)];
    let median = percentile(50);
    let mut deviations: Vec<_> = values.iter().map(|v| v.abs_diff(median)).collect();
    deviations.sort_unstable();
    let mean = values.iter().map(|v| *v as f64).sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|v| (*v as f64 - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    json!({"n":values.len(),"missing":missing,"min_ns":values[0],"p50_ns":median,"p90_ns":percentile(90),"p99_ns":percentile(99),"max_ns":values[values.len()-1],"mean_ns":mean,"population_stddev_ns":variance.sqrt(),"mad_ns":deviations[values.len().div_ceil(2)-1]})
}

/// Scripted fake backend for deterministic orchestration tests, never live evidence.
#[derive(Default)]
pub struct Fake {
    pub opened: usize,
    pub peak: usize,
    pub closes: usize,
    pub observed_peers: usize,
    pub fail_create: Option<usize>,
    pub fail_observation: Option<&'static str>,
    creates: usize,
}
impl Backend for Fake {
    type Session = bool;
    fn create(&mut self, _: &str, _: &str) -> Result<bool, String> {
        self.creates += 1;
        if self.fail_create == Some(self.creates) {
            return Err("scripted open failure".into());
        }
        self.opened += 1;
        self.peak = self.peak.max(self.opened);
        Ok(true)
    }
    fn observe(
        &mut self,
        _: &mut bool,
        stage: &str,
        peers: &mut [&mut bool],
    ) -> Result<Value, String> {
        assert!(peers.iter().all(|peer| **peer));
        self.observed_peers = self.observed_peers.max(peers.len());
        if self.fail_observation == Some(stage) {
            Err("scripted missing observation".into())
        } else {
            Ok(json!({"synthetic":true}))
        }
    }
    fn drive(&mut self, _: &mut bool) -> Result<(), String> {
        Ok(())
    }
    fn close(&mut self, session: &mut bool) {
        if *session {
            self.opened -= 1;
            self.closes += 1;
            *session = false;
        }
    }
    fn resources(&mut self) -> Value {
        json!({"synthetic_open_sessions":self.opened})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Small validator for the exact schema vocabulary used by this artifact.
    fn schema_matches(schema: &Value, value: &Value) -> bool {
        if let Some(one) = schema.get("oneOf") {
            return one
                .as_array()
                .unwrap()
                .iter()
                .filter(|s| schema_matches(s, value))
                .count()
                == 1;
        }
        if schema.get("const").is_some_and(|c| c != value) {
            return false;
        }
        if schema
            .get("enum")
            .is_some_and(|a| !a.as_array().unwrap().contains(value))
        {
            return false;
        }
        if let Some(kind) = schema.get("type").and_then(Value::as_str) {
            if !match kind {
                "object" => value.is_object(),
                "array" => value.is_array(),
                "string" => value.is_string(),
                "integer" => value.is_u64(),
                "null" => value.is_null(),
                _ => false,
            } {
                return false;
            }
        }
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            let Some(object) = value.as_object() else {
                return false;
            };
            if schema.get("required").is_some_and(|r| {
                r.as_array()
                    .unwrap()
                    .iter()
                    .any(|k| !object.contains_key(k.as_str().unwrap()))
            }) {
                return false;
            }
            if schema["additionalProperties"] == false
                && object.keys().any(|k| !properties.contains_key(k))
            {
                return false;
            }
            if properties
                .iter()
                .any(|(key, s)| object.get(key).is_some_and(|v| !schema_matches(s, v)))
            {
                return false;
            }
        }
        if let Some(items) = schema.get("items") {
            if !value
                .as_array()
                .unwrap()
                .iter()
                .all(|v| schema_matches(items, v))
            {
                return false;
            }
        }
        if schema
            .get("minimum")
            .is_some_and(|min| value.as_u64() < min.as_u64())
        {
            return false;
        }
        if schema.get("minLength").is_some_and(|min| {
            value.as_str().unwrap().len() < usize::try_from(min.as_u64().unwrap()).unwrap()
        }) {
            return false;
        }
        true
    }
    struct Tick(u64);
    impl Clock for Tick {
        fn now_ns(&mut self) -> u64 {
            self.0 += 10;
            self.0
        }
    }
    fn config() -> Config {
        Config {
            controller: "virtualgamepad.dualsense".into(),
            realization: "linux.uinput".into(),
            cycles: 2,
            concurrency: 4,
        }
    }
    #[test]
    fn deterministic_runner_schema_concurrency_and_cleanup() {
        let mut fake = Fake::default();
        let result = run(
            &mut fake,
            &mut Tick(0),
            &config(),
            &json!({"synthetic":true}),
        )
        .unwrap();
        assert_eq!(fake.peak, 4);
        assert_eq!(fake.observed_peers, 3);
        assert_eq!(fake.closes, 8);
        assert_eq!(fake.opened, 0);
        assert_eq!(
            result,
            run(
                &mut Fake::default(),
                &mut Tick(0),
                &config(),
                &json!({"synthetic":true})
            )
            .unwrap()
        );
        assert_eq!(
            serde_json::from_str::<Value>(&serde_json::to_string(&result).unwrap()).unwrap(),
            result
        );
        let schema: Value =
            serde_json::from_str(include_str!("../../acceptance/schema/run-v1.schema.json"))
                .unwrap();
        assert!(schema_matches(&schema, &result));
        let mut malformed = result.clone();
        malformed["samples"][0]["stages"]["api_call"]["reason"] =
            json!("contradicts measured value");
        assert!(!schema_matches(&schema, &malformed));
        assert_eq!(result["samples"].as_array().unwrap().len(), 8);
        for sample in result["samples"].as_array().unwrap() {
            assert_eq!(sample["stages"].as_object().unwrap().len(), STAGES.len());
            let mut previous = 0;
            for stage in STAGES {
                let at = sample["stages"][stage]["value_ns"].as_u64().unwrap();
                assert!(at >= previous);
                previous = at;
            }
        }
    }
    #[test]
    fn failures_are_samples_and_successful_siblings_always_close() {
        let mut fake = Fake {
            fail_create: Some(2),
            fail_observation: Some("first_neutral"),
            ..Fake::default()
        };
        let result = run(&mut fake, &mut Tick(0), &config(), &json!({})).unwrap();
        assert_eq!(result["samples"].as_array().unwrap().len(), 8);
        assert_eq!(fake.closes, 7);
        assert_eq!(fake.opened, 0);
        assert_eq!(result["samples"][1]["errors"][0], "scripted open failure");
        assert!(result["samples"][0]["stages"]["first_neutral"]["value_ns"].is_null());
        assert!(result["samples"][0]["stages"]["first_neutral"]["reason"].is_string());
    }
    #[test]
    fn nearest_rank_statistics_preserve_raw_distribution_and_missing_count() {
        let result = statistics(&[1, 2, 3, 4, 5], 2);
        assert_eq!(result["p50_ns"], 3);
        assert_eq!(result["p90_ns"], 5);
        assert_eq!(result["p99_ns"], 5);
        assert_eq!(result["mad_ns"], 1);
        assert_eq!(result["mean_ns"], 3.0);
        assert_eq!(result["missing"], 2);
        assert_eq!(statistics(&[], 3), json!({"n":0,"missing":3}));
    }
    #[test]
    fn missing_disappearance_is_an_error_without_leaking_fake_sessions() {
        let mut fake = Fake {
            fail_observation: Some("node_gone"),
            ..Fake::default()
        };
        let result = run(&mut fake, &mut Tick(0), &config(), &json!({})).unwrap();
        assert_eq!(fake.opened, 0);
        assert!(
            result["samples"]
                .as_array()
                .unwrap()
                .iter()
                .all(|s| !s["errors"].as_array().unwrap().is_empty())
        );
    }
    #[test]
    fn invalid_pair_or_workload_opens_nothing() {
        let mut fake = Fake::default();
        let mut c = config();
        c.realization = "linux.uhid.bluetooth".into();
        assert!(run(&mut fake, &mut Tick(0), &c, &json!({})).is_err());
        c = config();
        c.cycles = 0;
        assert!(run(&mut fake, &mut Tick(0), &c, &json!({})).is_err());
        assert_eq!(fake.creates, 0);
    }
}

#[cfg(test)]
mod peer_failure_tests {
    use super::*;
    struct OwnedSession {
        id: usize,
        open: bool,
        service: PeerService,
    }
    struct Peers {
        created: usize,
        open: usize,
        failed_stage: &'static str,
        failed_id: usize,
        failure_attempts: usize,
    }
    impl Backend for Peers {
        type Session = OwnedSession;
        fn create(&mut self, _: &str, _: &str) -> Result<OwnedSession, String> {
            let id = self.created;
            self.created += 1;
            self.open += 1;
            Ok(OwnedSession {
                id,
                open: true,
                service: PeerService::default(),
            })
        }
        fn observe(
            &mut self,
            _: &mut OwnedSession,
            stage: &str,
            peers: &mut [&mut OwnedSession],
        ) -> Result<Value, String> {
            if stage == self.failed_stage {
                for _ in 0..3 {
                    for peer in peers.iter_mut() {
                        peer.service.service(|| {
                            if peer.id == self.failed_id {
                                self.failure_attempts += 1;
                                Err(format!("session {} service failed", peer.id))
                            } else {
                                Ok(())
                            }
                        });
                    }
                }
            }
            Ok(json!({"synthetic": true}))
        }
        fn drive(&mut self, _: &mut OwnedSession) -> Result<(), String> {
            Ok(())
        }
        fn close(&mut self, session: &mut OwnedSession) {
            if session.open {
                self.open -= 1;
                session.open = false;
            }
        }
        fn take_service_errors(&mut self, session: &mut OwnedSession) -> Vec<String> {
            session.service.take_errors()
        }
        fn resources(&mut self) -> Value {
            json!({"open":self.open})
        }
    }
    struct Tick(u64);
    impl Clock for Tick {
        fn now_ns(&mut self) -> u64 {
            self.0 += 1;
            self.0
        }
    }
    #[test]
    fn peer_service_failure_belongs_to_its_sample_not_the_observed_sample() {
        for (stage, failed_id) in [("node_present", 0), ("node_gone", 1), ("sdl_ready", 0)] {
            let mut backend = Peers {
                created: 0,
                open: 0,
                failed_stage: stage,
                failed_id,
                failure_attempts: 0,
            };
            let config = Config {
                controller: "virtualgamepad.dualsense".into(),
                realization: "linux.uinput".into(),
                cycles: 1,
                concurrency: 3,
            };
            let result = run(&mut backend, &mut Tick(0), &config, &json!({})).unwrap();
            for index in 0..3 {
                assert_eq!(
                    result["samples"][index]["errors"],
                    if index == failed_id {
                        json!([format!("session {failed_id} service failed")])
                    } else {
                        json!([])
                    }
                );
                assert!(result["samples"][index]["stages"][stage]["value_ns"].is_u64());
            }
            assert_eq!(backend.failure_attempts, 1);
            assert_eq!(backend.open, 0);
        }
    }
}
