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
    // id can be either string or int, so leave it generic
    id: serde_json::Value,
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
    pub fn timing_buckets(&self) -> Histogram<u64> {
        let timings = self.timings();

        let mut h: Histogram<u64> = Histogram::new(timings.min_timestamp, timings.max_timestamp);

        let timestamp = |event: &TraceEvents| match event.ts {
            0 => None,
            ts => Some(ts),
        };

        self.trace_events
            .iter()
            .filter_map(timestamp)
            .for_each(|ts| {
                h.add_sample(ts);
            });

        h
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

// A simple generic Histogram.
// This didn't need to be generic but wanted to find out how generics work in
// rust.

// A histogram is an object that aggregates statistics of a series of incoming
// samples.
//
// It works by assigning incoming sample values are assigned to a bucket and
// increment that bucket count
//
// min, max: Minimum and maximum expected values. We use these values to compute
// the bucket size. TODO: allow creating histograme with known bucket size and
// without min and max!
//
// NUM_BUCKETS: Numer of buckets that the histogram uses. At the momemt this is
// a constant value (100).
//
// Bucket size: We create all buckets in equal size. Each bucket reperest a a
// range of values. Together they for a contigous range from [min, max]. We map
// values smaller than min to first bucket and values equal or larger than max
// to last bucket.
//
// For example if we have min=0, max=1000 with 100 buckets then ranges are: [0,
// 10), [10-20), .... [990, 1000]
use core::cmp::max;
use core::cmp::min;
use std::convert::{TryFrom, TryInto};
use std::ops::{DivAssign, SubAssign};

// Allow this to be passed into ctor.
const NUM_BUCKETS: usize = 100;

// The number of levels used to print the histograme
const NUM_LEVELS: i32 = 10;

#[derive(Debug)]
pub struct Histogram<T> {
    buckets: Vec<u64>, // count of entries in each buckets
    sample_count: u64, // total could of entries
    min: T,            // max value we expect
    max: T,            // min value we expect
    bucket_size: T,    // size of each bucket
}

impl<T> Histogram<T>
where
    T: Copy,
    T: DivAssign,
    T: SubAssign,
    T: TryFrom<usize>,
    T: TryInto<usize>,
{
    pub fn new(min: T, max: T) -> Histogram<T> {
        // compute bucket_size = (max - mix / num_bucket)
        // TODO: figure out how to use - and / directly instead of -= and /=
        // AFAICT, - and / return type is not necessarily T.
        let bucket_nums = match T::try_from(NUM_BUCKETS) {
            Ok(b) => b,
            _ => unreachable!(),
        };

        let mut bucket_size: T = max;
        bucket_size -= min;
        bucket_size /= bucket_nums;
        Histogram {
            buckets: vec![0; NUM_BUCKETS + 1 as usize],
            sample_count: 0,
            min,
            max,
            bucket_size,
        }
    }

    pub fn add_sample(&mut self, sample: T) {
        // compute bucket index = (sample - min) / bucket_size
        let mut bucket_index: T = sample;
        bucket_index -= self.min;
        bucket_index /= self.bucket_size;

        // convert into u8 so we can use an index to the buckets
        let bucket_index_u8: usize = match bucket_index.try_into() {
            Ok(b) => b,
            _ => unreachable!(),
        };

        let index = min(max(0, bucket_index_u8), NUM_BUCKETS);
        self.buckets[index] += 1;
        self.sample_count += 1;
    }

    // TODO: implement Display trait instead
    pub fn show(&self) -> String {
        // normalize sample counts
        let min = self.buckets.iter().min().unwrap_or(&0);
        let max = self.buckets.iter().max().unwrap_or(&1);
        let range = (max - min) as f64;
        // The quantization levels
        let levels = NUM_LEVELS;
        let normalized: Vec<f64> = self
            .buckets
            .iter()
            .map(|i| (i - min) as f64 / range)
            .map(|f| (f * levels as f64).round())
            .collect();

        let lines: String = (0..levels)
            .rev()
            .map(|l| {
                let line = normalized
                    .iter()
                    .map(|value| if value >= &f64::from(l) { '#' } else { ' ' })
                    .collect::<String>();
                return line;
            })
            .collect::<Vec<String>>()
            .join("\n");
        format!("{} samples:\n{}\n\n", self.sample_count, lines)
    }
}
