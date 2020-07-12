use std::fmt;

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
    ts: i64,
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
            "{:6} - {:10} ({:>2} thread): {:.40} ",
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
    pub fn timing(&self) -> (i64, i64){
        let min = self.trace_events.iter().map(|ref event| event.ts).min().unwrap_or(0);
        let max = self.trace_events.iter().map(|ref event| event.ts).max().unwrap_or(min);
        return (min, max);
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
            let label : String = metadata_events
                .iter()
                .find(|&event| event.pid == p.id && event.name == "process_labels")
                .map_or(String::new(), |&event| value_to_string(&event.args["labels"]));

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
            .filter(|&event|  filtered_process_ids.contains(&event.pid.to_string()))
            .cloned()
            .collect();

        Trace {
            metadata: self.metadata.to_owned(),
            trace_events: filtered_trace_events,
        }
    }
}

fn value_to_string(value : &serde_json::Value) -> String {
    match value.as_str() {
        Some(s) => String::from(s),
        None => String::new()
    }
}