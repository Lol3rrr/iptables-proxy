use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct ForwardingRoute {
    pub_ip: String,
    pub_port: u16,
    dest_ip: String,
    dest_port: u16,
    protocol: String,
}

impl ForwardingRoute {
    pub fn new(public: (String, u16), dest: (String, u16), protocol: String) -> Self {
        Self {
            pub_ip: public.0,
            pub_port: public.1,
            dest_ip: dest.0,
            dest_port: dest.1,
            protocol,
        }
    }

    pub fn public_ip(&self) -> &str {
        &self.pub_ip
    }
    pub fn public_port(&self) -> u16 {
        self.pub_port
    }

    fn register_args(&self) -> impl Iterator<Item = Vec<Cow<'static, str>>> {
        [
            vec![
                "-I".into(),
                "FORWARD".into(),
                "-d".into(),
                format!("{}", self.dest_ip).into(),
                "-m".into(),
                "comment".into(),
                "--comment".into(),
                "\"Accept to forward traffic\"".into(),
                "-m".into(),
                "tcp".into(),
                "-p".into(),
                self.protocol.clone().into(),
                "--dport".into(),
                format!("{}", self.pub_port).into(),
                "-j".into(),
                "ACCEPT".into(),
            ],
            vec![
                "-I".into(),
                "FORWARD".into(),
                "-m".into(),
                "comment".into(),
                "--comment".into(),
                "\"Accept to forward return traffic\"".into(),
                "-s".into(),
                format!("{}", self.dest_ip).into(),
                "-m".into(),
                "tcp".into(),
                "-p".into(),
                self.protocol.clone().into(),
                "--sport".into(),
                format!("{}", self.dest_port).into(),
                "-j".into(),
                "ACCEPT".into(),
            ],
            vec![
                "-t".into(),
                "nat".into(),
                "-I".into(),
                "PREROUTING".into(),
                "-m".into(),
                "tcp".into(),
                "-p".into(),
                self.protocol.clone().into(),
                "--dport".into(),
                format!("{}", self.pub_port).into(),
                "-m".into(),
                "comment".into(),
                "--comment".into(),
                "\"redirect pkts to homeserver\"".into(),
                "-j".into(),
                "DNAT".into(),
                "--to-destination".into(),
                format!("{}:{}", self.dest_ip, self.dest_port).into(),
            ],
        ]
        .into_iter()
    }
    fn deregister_args(&self) -> impl Iterator<Item = Vec<Cow<'static, str>>> {
        self.register_args().map(|mut args| {
            args.insert(0, "-D".into());
            args
        })
    }

    pub fn dry_register(&self) -> impl Iterator<Item = (String, String)> {
        self.register_args()
            .map(|args| ("iptables".to_string(), args.join(" ")))
    }

    pub fn register(&self) -> impl Iterator<Item = tokio::process::Command> {
        self.register_args().map(|args| {
            let mut cmd = tokio::process::Command::new("iptables");
            cmd.args(args.into_iter().map(|c| c.to_string()));
            cmd
        })
    }

    pub fn dry_deregister(&self) -> impl Iterator<Item = (String, String)> {
        self.deregister_args()
            .map(|args| ("iptables".to_string(), args.join(" ")))
    }
    pub fn deregister(&self) -> impl Iterator<Item = tokio::process::Command> {
        self.deregister_args().map(|args| {
            let mut cmd = tokio::process::Command::new("iptables");
            cmd.args(args.into_iter().map(|c| c.to_string()));
            cmd
        })
    }
}

pub struct Config {
    static_routes: Vec<ForwardingRoute>,
}

impl Config {
    pub fn load() -> Self {
        todo!("Work on loading the default standard Config")
    }
}
