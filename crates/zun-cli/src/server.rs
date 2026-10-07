// HTTP Server entry point — delegates all Axum logic to AxumBackend
use std::io::{self, Write};
use std::net::Ipv4Addr;
use std::process;
use std::thread;
use std::time::Duration;

use zun::interpreter::{HttpRoute, StaticRoute};
use zun::runtime::{
    ProgramState, RuntimeMode, backend::HttpBackend, execute_program_with_writer,
    load_context_and_program,
};

use crate::backends::axum_backend::{AxumBackend, validate_runtime_context};

/// Start HTTP server with the given path (main.zun file or directory)
pub fn run_serve(path: std::path::PathBuf, show_routertab: bool) {
    let (mut context, program) = match load_context_and_program(RuntimeMode::Serve, &path) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    print_startup_lines(&serve_banner_lines(), banner_line_delay());
    thread::sleep(banner_pause_duration());

    if let Some(loaded_config) = context.project_config() {
        print_startup_lines(
            &[cyan(format!(
                "Project  {} v{}",
                loaded_config.name, loaded_config.version
            ))],
            startup_line_delay(),
        );
    }

    print_startup_lines(
        &[cyan(format!("Mode     serve {}", program.path.display()))],
        startup_line_delay(),
    );
    print_startup_lines(&[String::new()], Duration::ZERO);

    let mut state = ProgramState::new();
    {
        let stdout = io::stdout();
        let mut output = StyledServeWriter::new(stdout.lock());
        match execute_program_with_writer(
            &program.statements,
            &mut state,
            &mut context,
            &mut output,
        ) {
            Ok(true) => {}
            Ok(false) => process::exit(0),
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    } // stdout lock is released here before the server starts

    if context.routes().is_empty() {
        print_startup_lines(&["No HTTP routes registered.".to_string()], Duration::ZERO);
        return;
    }

    if let Err(err) = validate_runtime_context(&context) {
        eprintln!("{err}");
        process::exit(1);
    }

    let host = context.server_host().to_string();
    let port = context.server_port();

    print_startup_lines(
        &[dim(format!(
            "Summary  {} HTTP route(s), {} static route(s)",
            context.routes().len(),
            context.static_routes().len()
        ))],
        startup_line_delay(),
    );

    if show_routertab {
        print_startup_lines(&[String::new()], Duration::ZERO);
        print_startup_lines(
            &route_output_lines(show_routertab, context.routes(), context.static_routes()),
            route_table_line_delay(),
        );
        print_startup_lines(&[String::new()], Duration::ZERO);
    }

    print_startup_lines(&server_address_lines(&host, port), startup_line_delay());

    // Hand off all routes to the backend
    let mut backend = AxumBackend::new();
    for route in context.routes().iter().cloned() {
        backend.register_route(route);
    }
    for sr in context.static_routes().iter().cloned() {
        backend.register_static(sr);
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    if let Err(err) = rt.block_on(backend.serve(context, &host, port)) {
        eprintln!("{err}");
        process::exit(1);
    }
}

fn serve_banner_lines() -> Vec<String> {
    let color = "\u{1b}[38;5;43m";
    let reset = "\u{1b}[0m";
    [
        "███████╗██╗   ██╗███╗   ██╗",
        "╚══███╔╝██║   ██║████╗  ██║",
        "  ███╔╝ ██║   ██║██╔██╗ ██║",
        " ███╔╝  ██║   ██║██║╚██╗██║",
        "███████╗╚██████╔╝██║ ╚████║",
        "╚══════╝ ╚═════╝ ╚═╝  ╚═══╝",
    ]
    .into_iter()
    .map(|line| format!("{color}{line}{reset}"))
    .collect()
}

fn print_startup_lines(lines: &[String], delay: Duration) {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    for line in lines {
        writeln!(lock, "{line}").ok();
        lock.flush().ok();
        if !delay.is_zero() {
            thread::sleep(delay);
        }
    }
}

fn banner_line_delay() -> Duration {
    Duration::from_millis(18)
}

fn banner_pause_duration() -> Duration {
    Duration::from_millis(500)
}

fn startup_line_delay() -> Duration {
    Duration::from_millis(90)
}

fn route_table_line_delay() -> Duration {
    Duration::from_millis(14)
}

struct StyledServeWriter<W> {
    inner: W,
    pending: Vec<u8>,
}

impl<W> StyledServeWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            pending: Vec::new(),
        }
    }
}

impl<W: Write> Write for StyledServeWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(buf);
        while let Some(pos) = self.pending.iter().position(|byte| *byte == b'\n') {
            let line = self.pending.drain(..=pos).collect::<Vec<_>>();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_end_matches('\n');
            let styled = style_runtime_line(line);
            self.inner.write_all(styled.as_bytes())?;
            self.inner.write_all(b"\n")?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.pending.is_empty() {
            let line = String::from_utf8_lossy(&self.pending).to_string();
            let styled = style_runtime_line(line.trim_end_matches('\n'));
            self.inner.write_all(styled.as_bytes())?;
            self.pending.clear();
        }
        self.inner.flush()
    }
}

fn server_address_lines(host: &str, port: u16) -> Vec<String> {
    let networks = if host == "0.0.0.0" {
        discover_lan_ipv4_addrs()
            .into_iter()
            .map(|addr| addr.to_string())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    server_address_lines_for_networks(host, port, &networks)
}

fn server_address_lines_for_networks(host: &str, port: u16, networks: &[String]) -> Vec<String> {
    if host == "0.0.0.0" {
        let mut lines = vec![format!(
            "{}   http://127.0.0.1:{port}",
            green("  ➜  Local:")
        )];
        let mut networks = networks.to_vec();
        networks.sort();
        networks.dedup();

        if networks.is_empty() {
            lines.push(format!(
                "{} listening on all interfaces (:{}), use your LAN IP to access",
                cyan("  ➜  Network:"),
                port
            ));
        } else {
            for network in networks {
                lines.push(format!(
                    "{} http://{}:{port}",
                    cyan("  ➜  Network:"),
                    network
                ));
            }
        }
        lines
    } else {
        vec![format!("{}   http://{host}:{port}", green("  ➜  Local:"))]
    }
}

fn discover_lan_ipv4_addrs() -> Vec<Ipv4Addr> {
    #[cfg(unix)]
    {
        discover_lan_ipv4_addrs_unix()
    }

    #[cfg(windows)]
    {
        discover_lan_ipv4_addrs_windows()
    }

    #[cfg(not(any(unix, windows)))]
    {
        Vec::new()
    }
}

fn collect_visible_lan_ipv4_addrs(addrs: impl IntoIterator<Item = Ipv4Addr>) -> Vec<Ipv4Addr> {
    use std::collections::BTreeSet;

    addrs
        .into_iter()
        .filter(|ip| is_visible_lan_ipv4(*ip))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(unix)]
fn discover_lan_ipv4_addrs_unix() -> Vec<Ipv4Addr> {
    use std::ptr;

    let mut addrs = Vec::new();
    let mut ifaddrs: *mut libc::ifaddrs = ptr::null_mut();

    unsafe {
        if libc::getifaddrs(&mut ifaddrs) != 0 {
            return Vec::new();
        }

        let mut current = ifaddrs;
        while !current.is_null() {
            let addr = (*current).ifa_addr;
            if !addr.is_null() && (*addr).sa_family as i32 == libc::AF_INET {
                let sockaddr = &*(addr as *const libc::sockaddr_in);
                let ip = Ipv4Addr::from(u32::from_be(sockaddr.sin_addr.s_addr));
                addrs.push(ip);
            }
            current = (*current).ifa_next;
        }

        libc::freeifaddrs(ifaddrs);
    }

    collect_visible_lan_ipv4_addrs(addrs)
}

#[cfg(windows)]
fn discover_lan_ipv4_addrs_windows() -> Vec<Ipv4Addr> {
    use std::mem::size_of;
    use std::ptr;
    use windows_sys::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
        GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows_sys::Win32::Networking::WinSock::{AF_INET, SOCKADDR, SOCKADDR_IN};

    let mut addrs = Vec::new();
    let mut buffer_len: u32 = 15 * 1024;

    for _ in 0..3 {
        let mut buffer = vec![0u8; buffer_len as usize];
        let result = unsafe {
            GetAdaptersAddresses(
                AF_INET as u32,
                GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER,
                ptr::null(),
                buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
                &mut buffer_len,
            )
        };

        if result == ERROR_BUFFER_OVERFLOW {
            continue;
        }

        if result != 0 {
            return Vec::new();
        }

        let mut current = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
        unsafe {
            while !current.is_null() {
                let mut unicast = (*current).FirstUnicastAddress;
                while !unicast.is_null() {
                    let socket_address = (*unicast).Address.lpSockaddr;
                    if !socket_address.is_null()
                        && (*socket_address).sa_family == AF_INET
                        && (*unicast).Address.iSockaddrLength as usize >= size_of::<SOCKADDR_IN>()
                    {
                        let sockaddr = &*(socket_address as *const SOCKADDR as *const SOCKADDR_IN);
                        let ip = Ipv4Addr::from(u32::from_be(sockaddr.sin_addr.S_un.S_addr));
                        addrs.push(ip);
                    }
                    unicast = (*unicast).Next;
                }
                current = (*current).Next;
            }
        }

        return collect_visible_lan_ipv4_addrs(addrs);
    }

    Vec::new()
}

fn is_visible_lan_ipv4(ip: Ipv4Addr) -> bool {
    !ip.is_loopback() && !ip.is_unspecified() && is_private_lan_ipv4(ip)
}

fn is_private_lan_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, _, _] = ip.octets();
    matches!((a, b), (10, _) | (172, 16..=31) | (192, 168))
}

fn route_output_lines(
    show_routertab: bool,
    routes: &[HttpRoute],
    static_routes: &[StaticRoute],
) -> Vec<String> {
    if !show_routertab {
        return Vec::new();
    }

    render_route_table(routes, static_routes)
}

fn render_route_table(routes: &[HttpRoute], static_routes: &[StaticRoute]) -> Vec<String> {
    let rows = build_route_rows(routes, static_routes);
    let mut type_width = "Type".len();
    let mut method_width = "Method".len();
    let mut path_width = "Path".len();
    let mut target_width = "Target".len();

    for row in &rows {
        type_width = type_width.max(row.kind.len());
        method_width = method_width.max(row.method.len());
        path_width = path_width.max(row.path.len());
        target_width = target_width.max(row.target.len());
    }

    let border = format!(
        "+-{}-+-{}-+-{}-+-{}-+",
        "-".repeat(type_width),
        "-".repeat(method_width),
        "-".repeat(path_width),
        "-".repeat(target_width)
    );

    let mut lines = vec![
        cyan("Routes".to_string()),
        border.clone(),
        format!(
            "| {} | {} | {} | {} |",
            pad_cell("Type", type_width),
            pad_cell("Method", method_width),
            pad_cell("Path", path_width),
            pad_cell("Target", target_width)
        ),
        border.clone(),
    ];

    for row in rows {
        lines.push(format!(
            "| {} | {} | {} | {} |",
            style_kind(&row.kind, type_width),
            style_method(&row.method, method_width),
            pad_cell(&row.path, path_width),
            dim(pad_cell(&row.target, target_width))
        ));
    }

    lines.push(border);
    lines
}

fn build_route_rows(routes: &[HttpRoute], static_routes: &[StaticRoute]) -> Vec<RouteRow> {
    let mut rows = Vec::with_capacity(routes.len() + static_routes.len());
    for route in routes {
        rows.push(RouteRow {
            kind: "HTTP".to_string(),
            method: route.method.clone(),
            path: route.path.clone(),
            target: route.name.clone(),
        });
    }
    for route in static_routes {
        rows.push(RouteRow {
            kind: "STATIC".to_string(),
            method: "--".to_string(),
            path: route.url_prefix.clone(),
            target: route.module_path.clone(),
        });
    }
    rows
}

fn style_runtime_line(line: &str) -> String {
    if let Some(rest) = line.strip_prefix("[ERROR]") {
        format!("{}{}", error_badge(), rest)
    } else {
        line.to_string()
    }
}

fn style_kind(kind: &str, width: usize) -> String {
    match kind {
        "STATIC" => cyan(pad_cell(kind, width)),
        _ => blue(pad_cell(kind, width)),
    }
}

fn style_method(method: &str, width: usize) -> String {
    let padded = pad_cell(method, width);
    match method {
        "GET" => green(padded),
        "POST" => blue(padded),
        "PUT" => yellow(padded),
        "DELETE" => red(padded),
        "PATCH" => magenta(padded),
        _ => dim(padded),
    }
}

fn pad_cell(value: &str, width: usize) -> String {
    format!("{value:<width$}")
}

fn green(value: impl AsRef<str>) -> String {
    format!("\u{1b}[32m{}\u{1b}[0m", value.as_ref())
}

fn cyan(value: impl AsRef<str>) -> String {
    format!("\u{1b}[36m{}\u{1b}[0m", value.as_ref())
}

fn blue(value: impl AsRef<str>) -> String {
    format!("\u{1b}[34m{}\u{1b}[0m", value.as_ref())
}

fn yellow(value: impl AsRef<str>) -> String {
    format!("\u{1b}[33m{}\u{1b}[0m", value.as_ref())
}

fn red(value: impl AsRef<str>) -> String {
    format!("\u{1b}[31m{}\u{1b}[0m", value.as_ref())
}

fn error_badge() -> &'static str {
    "\u{1b}[97;41m[ERROR]\u{1b}[0m"
}

fn magenta(value: impl AsRef<str>) -> String {
    format!("\u{1b}[35m{}\u{1b}[0m", value.as_ref())
}

fn dim(value: impl AsRef<str>) -> String {
    format!("\u{1b}[2m{}\u{1b}[0m", value.as_ref())
}

#[derive(Debug)]
struct RouteRow {
    kind: String,
    method: String,
    path: String,
    target: String,
}

#[cfg(test)]
mod tests {
    use super::{
        banner_line_delay, banner_pause_duration, collect_visible_lan_ipv4_addrs,
        route_output_lines, route_table_line_delay, serve_banner_lines,
        server_address_lines_for_networks, startup_line_delay, style_runtime_line,
    };
    use std::net::Ipv4Addr;
    use std::sync::Arc;
    use std::time::Duration;
    use zun::interpreter::{HttpRoute, RouteModuleState, StaticRoute};

    #[test]
    fn serve_banner_lines_render_zun_ascii_banner() {
        let lines = serve_banner_lines();
        assert_eq!(lines.len(), 6);
        assert_eq!(
            lines[0],
            "\u{1b}[38;5;43m███████╗██╗   ██╗███╗   ██╗\u{1b}[0m"
        );
        assert_eq!(
            lines[5],
            "\u{1b}[38;5;43m╚══════╝ ╚═════╝ ╚═╝  ╚═══╝\u{1b}[0m"
        );
    }

    #[test]
    fn banner_pause_duration_is_half_second() {
        assert_eq!(banner_pause_duration(), Duration::from_millis(500));
    }

    #[test]
    fn startup_output_delays_stay_lightweight() {
        assert_eq!(banner_line_delay(), Duration::from_millis(18));
        assert_eq!(startup_line_delay(), Duration::from_millis(90));
        assert_eq!(route_table_line_delay(), Duration::from_millis(14));
    }

    #[test]
    fn server_address_lines_render_detected_lan_addresses_for_wildcard_bind() {
        assert_eq!(
            server_address_lines_for_networks(
                "0.0.0.0",
                8080,
                &[
                    "192.168.1.25".to_string(),
                    "10.0.0.42".to_string(),
                    "192.168.1.25".to_string(),
                ]
            ),
            vec![
                "\u{1b}[32m  ➜  Local:\u{1b}[0m   http://127.0.0.1:8080".to_string(),
                "\u{1b}[36m  ➜  Network:\u{1b}[0m http://10.0.0.42:8080".to_string(),
                "\u{1b}[36m  ➜  Network:\u{1b}[0m http://192.168.1.25:8080".to_string(),
            ]
        );
    }

    #[test]
    fn server_address_lines_fall_back_when_no_lan_address_is_available() {
        assert_eq!(
            server_address_lines_for_networks("0.0.0.0", 8080, &[]),
            vec![
                "\u{1b}[32m  ➜  Local:\u{1b}[0m   http://127.0.0.1:8080".to_string(),
                "\u{1b}[36m  ➜  Network:\u{1b}[0m listening on all interfaces (:8080), use your LAN IP to access".to_string(),
            ]
        );
    }

    #[test]
    fn server_address_lines_preserve_specific_host() {
        assert_eq!(
            server_address_lines_for_networks("127.0.0.1", 8080, &["192.168.1.25".to_string()]),
            vec!["\u{1b}[32m  ➜  Local:\u{1b}[0m   http://127.0.0.1:8080".to_string()]
        );
    }

    #[test]
    fn collect_visible_lan_ipv4_addrs_filters_non_private_and_deduplicates() {
        let addrs = collect_visible_lan_ipv4_addrs([
            Ipv4Addr::new(127, 0, 0, 1),
            Ipv4Addr::new(10, 0, 0, 42),
            Ipv4Addr::new(10, 0, 0, 42),
            Ipv4Addr::new(172, 18, 36, 234),
            Ipv4Addr::new(192, 168, 1, 9),
            Ipv4Addr::new(8, 8, 8, 8),
        ]);

        assert_eq!(
            addrs,
            vec![
                Ipv4Addr::new(10, 0, 0, 42),
                Ipv4Addr::new(172, 18, 36, 234),
                Ipv4Addr::new(192, 168, 1, 9),
            ]
        );
    }

    #[test]
    fn route_output_lines_are_hidden_without_routertab() {
        let routes = vec![sample_http_route()];
        let static_routes = vec![sample_static_route()];
        assert!(route_output_lines(false, &routes, &static_routes).is_empty());
    }

    #[test]
    fn route_output_lines_render_table_with_borders() {
        let routes = vec![sample_http_route()];
        let static_routes = vec![sample_static_route()];

        let lines = route_output_lines(true, &routes, &static_routes);
        let plain = strip_ansi_lines(&lines);

        assert_eq!(plain[0], "Routes");
        assert!(plain[1].starts_with("+-"));
        assert_eq!(plain[2], "| Type   | Method | Path       | Target     |");
        assert_eq!(plain[4], "| HTTP   | GET    | /api/users | get_users  |");
        assert_eq!(plain[5], "| STATIC | --     | /assets    | app/public |");
    }

    #[test]
    fn style_runtime_line_colors_info_and_error_prefixes() {
        assert_eq!(
            style_runtime_line("[INFO] server starting"),
            "[INFO] server starting"
        );
        assert_eq!(
            style_runtime_line("[ERROR] boom"),
            "\u{1b}[97;41m[ERROR]\u{1b}[0m boom"
        );
    }

    fn sample_http_route() -> HttpRoute {
        HttpRoute {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            name: "get_users".to_string(),
            params: Vec::new(),
            variadic_param: None,
            return_type: None,
            cors: None,
            parent_cors: None,
            response_headers: Vec::new(),
            body: Vec::new(),
            module_state: Arc::new(RouteModuleState::default()),
        }
    }

    fn sample_static_route() -> StaticRoute {
        StaticRoute {
            url_prefix: "/assets".to_string(),
            module_path: "app/public".to_string(),
            base_dir: "app".to_string(),
        }
    }

    fn strip_ansi_lines(lines: &[String]) -> Vec<String> {
        lines.iter().map(|line| strip_ansi(line)).collect()
    }

    fn strip_ansi(value: &str) -> String {
        let mut result = String::new();
        let mut chars = value.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\u{1b}' {
                while let Some(next) = chars.next() {
                    if next == 'm' {
                        break;
                    }
                }
            } else {
                result.push(ch);
            }
        }
        result
    }
}
