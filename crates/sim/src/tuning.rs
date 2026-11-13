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

    /// Velocidad del impulso del golpe (px/s).
    pub strike_speed: f32,
    /// Empuje adicional del stick en el impacto (px/s). 0 = impulso
    /// exactamente opuesto al golpe.
    pub strike_steer: f32,
    /// Tiempo sin gravedad ni control tras el impacto (s).
    pub strike_float: f32,
    /// Ventana activa del golpe: si un objeto entra en la caja durante este
    /// tiempo tras pulsar, el golpe conecta (s).
    pub strike_window: f32,
    /// Tiempo tras un golpe fallido antes de poder golpear otra vez (s).
    pub strike_cooldown: f32,
    /// Buffer de golpe: una pulsación que aún no puede empezar espera esto (s).
    pub strike_buffer: f32,
    /// Distancia del centro del jugador al centro de la caja de golpe (px).
    pub strike_reach: f32,
    /// Lado de la caja de golpe (px).
    pub strike_size: f32,
    /// Golpes disponibles tras recargar.
    pub max_strikes: f32,
    /// Tiempo antes de poder golpear de nuevo el mismo objeto (s).
    pub object_cooldown: f32,
    /// Tiempo hasta que una roca frágil reaparece (s).
    pub fragile_respawn: f32,
    /// Multiplicadores de impulso por tipo de objeto.
    pub bone_mult: f32,
    pub moss_mult: f32,
    /// Hit-stop y temblor de pantalla en el impacto (s).
    pub hitstop: f32,
    pub shake_time: f32,

    /// Velocidad del escarabajo (px/s) y tiempo que queda aturdido (s).
    pub beetle_speed: f32,
    pub beetle_stun: f32,
    /// Velocidad máxima del murciélago (px/s).
    pub bat_speed: f32,
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
            strike_speed: 210.0,
            strike_steer: 0.0,
            strike_float: 0.06,
            strike_window: 0.07,
            strike_cooldown: 0.15,
            strike_buffer: 0.1,
            strike_reach: 9.0,
            strike_size: 16.0,
            max_strikes: 1.0,
            object_cooldown: 0.35,
            fragile_respawn: 2.5,
            bone_mult: 1.35,
            moss_mult: 0.7,
            hitstop: 0.05,
            shake_time: 0.12,
            beetle_speed: 20.0,
            beetle_stun: 1.5,
            bat_speed: 30.0,
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
            "strike_speed" => &mut self.strike_speed,
            "strike_steer" => &mut self.strike_steer,
            "strike_float" => &mut self.strike_float,
            "strike_window" => &mut self.strike_window,
            "strike_cooldown" => &mut self.strike_cooldown,
            "strike_buffer" => &mut self.strike_buffer,
            "strike_reach" => &mut self.strike_reach,
            "strike_size" => &mut self.strike_size,
            "max_strikes" => &mut self.max_strikes,
            "object_cooldown" => &mut self.object_cooldown,
            "fragile_respawn" => &mut self.fragile_respawn,
            "bone_mult" => &mut self.bone_mult,
            "moss_mult" => &mut self.moss_mult,
            "hitstop" => &mut self.hitstop,
            "shake_time" => &mut self.shake_time,
            "beetle_speed" => &mut self.beetle_speed,
            "beetle_stun" => &mut self.beetle_stun,
            "bat_speed" => &mut self.bat_speed,
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
