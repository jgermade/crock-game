/// Constantes de movimiento. Unidades: píxeles de juego y segundos.
///
/// Se pueden recargar en caliente desde un fichero `clave = valor`
/// (ver `assets/tuning.cfg`).
#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    /// Velocidad horizontal máxima corriendo (px/s).
    pub run_speed: f32,
    /// Aceleración hacia `run_speed` (px/s²).
    pub run_accel: f32,
    /// Deceleración cuando se supera `run_speed` en la dirección pulsada (px/s²).
    pub run_reduce: f32,
    /// Multiplicador de control horizontal en el aire.
    pub air_mult: f32,
    /// Gravedad (px/s²).
    pub gravity: f32,
    /// Velocidad máxima de caída (px/s).
    pub max_fall: f32,
    /// Velocidad máxima de caída manteniendo abajo (px/s).
    pub fast_max_fall: f32,
    /// Velocidad vertical inicial del salto (px/s, positiva = hacia arriba).
    pub jump_speed: f32,
    /// Impulso horizontal extra al saltar con dirección pulsada (px/s).
    pub jump_h_boost: f32,
    /// Tiempo durante el que mantener el botón sostiene la velocidad de salto (s).
    pub var_jump_time: f32,
    /// Por debajo de esta |vy| la gravedad se reduce a la mitad si se mantiene salto (px/s).
    pub half_grav_threshold: f32,
    /// Coyote time: margen para saltar tras dejar el suelo (s).
    pub coyote_time: f32,
    /// Input buffer: margen para un salto pulsado antes de tocar suelo (s).
    pub jump_buffer: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            run_speed: 90.0,
            run_accel: 1000.0,
            run_reduce: 400.0,
            air_mult: 0.65,
            gravity: 900.0,
            max_fall: 160.0,
            fast_max_fall: 240.0,
            jump_speed: 105.0,
            jump_h_boost: 40.0,
            var_jump_time: 0.2,
            half_grav_threshold: 40.0,
            coyote_time: 0.1,
            jump_buffer: 0.09,
        }
    }
}

impl Tuning {
    fn field_mut(&mut self, key: &str) -> Option<&mut f32> {
        Some(match key {
            "run_speed" => &mut self.run_speed,
            "run_accel" => &mut self.run_accel,
            "run_reduce" => &mut self.run_reduce,
            "air_mult" => &mut self.air_mult,
            "gravity" => &mut self.gravity,
            "max_fall" => &mut self.max_fall,
            "fast_max_fall" => &mut self.fast_max_fall,
            "jump_speed" => &mut self.jump_speed,
            "jump_h_boost" => &mut self.jump_h_boost,
            "var_jump_time" => &mut self.var_jump_time,
            "half_grav_threshold" => &mut self.half_grav_threshold,
            "coyote_time" => &mut self.coyote_time,
            "jump_buffer" => &mut self.jump_buffer,
            _ => return None,
        })
    }

    /// Parsea `clave = valor` por línea, partiendo de los valores por defecto.
    /// `#` inicia un comentario. Devuelve también los avisos encontrados.
    pub fn parse(src: &str) -> (Self, Vec<String>) {
        let mut t = Self::default();
        let mut warnings = Vec::new();
        for (n, line) in src.lines().enumerate() {
            let line = line.split('#').next().unwrap().trim();
            if line.is_empty() {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                warnings.push(format!("línea {}: falta '='", n + 1));
                continue;
            };
            let (k, v) = (k.trim(), v.trim());
            match (t.field_mut(k), v.parse::<f32>()) {
                (Some(f), Ok(v)) if v.is_finite() => *f = v,
                (None, _) => warnings.push(format!("línea {}: clave desconocida '{k}'", n + 1)),
                _ => warnings.push(format!("línea {}: valor inválido '{v}'", n + 1)),
            }
        }
        (t, warnings)
    }
}
