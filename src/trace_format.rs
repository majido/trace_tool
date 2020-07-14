use std::fmt;
use std::time::Duration;

// Trace structures

// Generated using: https://transform.now.sh/json-to-rust-serde
// Metadata has lots of fileds and they change over time. So keep it a generic
// JSON value.
type Metadata = serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct TraceEvents {
    pid: i64,
    tid: i64,
    ts: u64,
    ph: String,
    cat: String,
    #[serde(default)]
    name: String,
    // args can contain many different field so keep is a generic JSON value
    args: serde_json::Value,
    dur: i64,
    tdur: i64,
    tts: i64,
    s: String,
    id: String,
    scope: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Trace {
    #[serde(rename = "traceEvents")]
    pub trace_events: Vec<TraceEvents>,
    pub metadata: Metadata,
}

// Represent a process based on the data from the trace
#[derive(Debug)]
pub struct ProcessInfo {
    pub id: i64,
    pub name: String,
    label: String,
    threads: Vec<String>,
}

#[derive(Debug)]
pub struct Timing {
    pub duration: Duration,
    pub min_timestamp: u64,
    pub max_timestamp: u64,
}

impl ProcessInfo {
    fn name_rank(&self) -> i8 {
        match self.name.as_ref() {
            "Renderer" => 1,
            _ => 0,
        }
    }

    pub fn is_renderer(&self) -> bool {
        self.name == "Renderer"
    }
}
impl fmt::Display for ProcessInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:6} - {:10} ({:>2} thread): {:.40}",
            self.id,
            self.name,
            self.threads.len(),
            self.label
        )
    }
}

impl Trace {
    pub fn info(&self) -> String {
        return format!(
            "captured={}, version={}",
            value_to_string(&self.metadata["trace-capture-datetime"]),
            value_to_string(&self.metadata["product-version"])
        );
    }

    pub fn metadata_events(&self) -> Vec<&TraceEvents> {
        return self
            .trace_events
            .iter()
            .filter(|&event| event.cat == "__metadata")
            .collect();
    }

    // return a tuple that contain timestamp of earliest and latest events
    fn timestamp_range(&self) -> (u64, u64) {
        // Ignore zero timestamps since they don't add any value
        let timestamp = |event: &TraceEvents| match event.ts {
            0 => None,
            ts => Some(ts),
        };

        let min = self
            .trace_events
            .iter()
            .filter_map(timestamp)
            .min()
            .unwrap_or(0);
        let max = self
            .trace_events
            .iter()
            .filter_map(timestamp)
            .max()
            .unwrap_or(min);
        return (min, max);
    }

    pub fn timings(&self) -> Timing {
        let range = self.timestamp_range();

        Timing {
            duration: Duration::from_micros(range.1 - range.0),
            min_timestamp: range.0,
            max_timestamp: range.1,
        }
    }

    // Return a vector where each entry 
    pub fn timing_buckets(&self) -> Vec<u64>{
        const NUM_BUCKETS: usize = 100;
        let mut buckets = vec![0; NUM_BUCKETS];

        let timings = self.timings();

        let bucket_size: u64 = ((timings.max_timestamp - timings.min_timestamp) as f64 / NUM_BUCKETS as f64).ceil() as u64;

        println!("{}", bucket_size);

        let timestamp = |event: &TraceEvents| match event.ts {
            0 => None,
            ts => Some(ts),
        };

        self.trace_events
        .iter()
        .filter_map(timestamp)
        .for_each(|ts| {
            buckets[((ts - timings.min_timestamp)/ bucket_size) as usize] += 1
        });

        buckets
    }

    pub fn processes(&self) -> Vec<ProcessInfo> {
        let metadata_events = self.metadata_events();

        let mut processes: Vec<ProcessInfo> = metadata_events
            .iter()
            .filter(|&event| event.name == "process_name")
            .map(|&event| ProcessInfo {
                id: event.pid,
                name: value_to_string(&event.args["name"]),
                label: String::new(),
                threads: vec![],
            })
            .collect();

        // TODO: doing multiple loops is inefficient.
        println!("{}", metadata_events.len());
        for p in &mut processes {
            let label: String = metadata_events
                .iter()
                .find(|&event| event.pid == p.id && event.name == "process_labels")
                .map_or(String::new(), |&event| {
                    value_to_string(&event.args["labels"])
                });

            let threads: Vec<String> = metadata_events
                .iter()
                .filter(|&event| event.pid == p.id && event.name == "thread_name")
                .map(|&event| value_to_string(&event.args["name"]))
                .collect();

            p.label = label;
            p.threads = threads;
        }

        processes.sort_by(|a, b| a.name_rank().cmp(&b.name_rank()));
        return processes;
    }

    // Create a new trace that only includes events from the given process
    pub fn filter(&self, filtered_process_ids: Vec<String>) -> Trace {
        let filtered_trace_events = self
            .trace_events
            .iter()
            .filter(|&event| filtered_process_ids.contains(&event.pid.to_string()))
            .cloned()
            .collect();

        Trace {
            metadata: self.metadata.to_owned(),
            trace_events: filtered_trace_events,
        }
    }
}

fn value_to_string(value: &serde_json::Value) -> String {
    match value.as_str() {
        Some(s) => String::from(s),
        None => String::new(),
    }
}
