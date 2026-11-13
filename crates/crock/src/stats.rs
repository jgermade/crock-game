//! Métricas de frame pacing y latencia.

pub const HISTORY: usize = 240;

pub struct Stats {
    /// Intervalo entre presentaciones consecutivas (ns).
    pub frame_ns: [u64; HISTORY],
    /// Trabajo de CPU del frame: input + simulación + render, sin present (ns).
    pub work_ns: [u64; HISTORY],
    pub head: usize,
    /// Latencia pulsación → retorno de present (ns), últimas muestras.
    pub latency_ns: [u64; 32],
    pub latency_count: usize,
    pub ticks_this_frame: u32,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            frame_ns: [0; HISTORY],
            work_ns: [0; HISTORY],
            head: 0,
            latency_ns: [0; 32],
            latency_count: 0,
            ticks_this_frame: 0,
        }
    }

    pub fn push_frame(&mut self, frame: u64, work: u64) {
        self.frame_ns[self.head] = frame;
        self.work_ns[self.head] = work;
        self.head = (self.head + 1) % HISTORY;
    }

    pub fn push_latency(&mut self, ns: u64) {
        self.latency_ns[self.latency_count % self.latency_ns.len()] = ns;
        self.latency_count += 1;
    }

    /// (media, peor) de los intervalos de frame registrados, en ms.
    pub fn frame_ms(&self) -> (f64, f64) {
        avg_max(&self.frame_ns)
    }

    pub fn work_ms(&self) -> (f64, f64) {
        avg_max(&self.work_ns)
    }

    /// (última, media) de latencia pulsación→present en ms.
    pub fn latency_ms(&self) -> Option<(f64, f64)> {
        if self.latency_count == 0 {
            return None;
        }
        let n = self.latency_count.min(self.latency_ns.len());
        let last = self.latency_ns[(self.latency_count - 1) % self.latency_ns.len()];
        let avg = self.latency_ns[..n].iter().sum::<u64>() as f64 / n as f64;
        Some((last as f64 / 1e6, avg / 1e6))
    }
}

fn avg_max(v: &[u64]) -> (f64, f64) {
    let valid: Vec<u64> = v.iter().copied().filter(|&x| x > 0).collect();
    if valid.is_empty() {
        return (0.0, 0.0);
    }
    let avg = valid.iter().sum::<u64>() as f64 / valid.len() as f64;
    let max = *valid.iter().max().unwrap();
    (avg / 1e6, max as f64 / 1e6)
}
