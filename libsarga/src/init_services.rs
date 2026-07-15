use crate::init::Service;

pub static DEFAULT_SERVICES: &[Service] = &[
    Service {
        name: "udev",
        command: "/bin/udevd",
        depends: &[],
        respawn: true,
        respawn_delay_ms: 1000,
    },
    Service {
        name: "net",
        command: "/bin/netd",
        depends: &["udev"],
        respawn: true,
        respawn_delay_ms: 2000,
    },
    Service {
        name: "searchd",
        command: "/bin/searchd",
        depends: &[],
        respawn: true,
        respawn_delay_ms: 3000,
    },
    Service {
        name: "gui",
        command: "/bin/shell",
        depends: &["udev", "net", "searchd"],
        respawn: true,
        respawn_delay_ms: 500,
    },
];
