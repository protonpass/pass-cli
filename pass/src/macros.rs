#[macro_export]
macro_rules! map (
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
     };
);

#[macro_export]
macro_rules! display_for_basic {
    ($t:ty) => {
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

#[macro_export]
macro_rules! assert_response {
    ($res:expr) => {{
        if !$res.status().is_success() {
            $crate::utils::debug_response(&$res);
            return Err(anyhow::anyhow!("Invalid status code: {}", $res.status()));
        }

        match $res.body_json() {
            Ok(v) => v,
            Err(e) => {
                $crate::utils::debug_response(&$res);
                return Err(anyhow::anyhow!("Error parsing response body: {}", e));
            }
        }
    }};
}
