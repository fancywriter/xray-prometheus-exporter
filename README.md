# Xray statistics exporter to Prometheus
This simple application exports user statistics provided by [Xray](https://github.com/XTLS/Xray-core) in Prometheus format.

## Configuration
Assume you have the following Configuration on your Xray server (minimal configuration with Reality for example)
```json
{
    "inbounds": [
        {
            "port": 443,
            "protocol": "vless",
            "tag": "vless_tls",
            "settings": {
                "clients": [
                    {
                        "id": "<uuid of client 1>",
                        "flow": "xtls-rprx-vision",
                        "email": "client1"
                    },
                    {
                        "id": "<uuid of client 2>",
                        "flow": "xtls-rprx-vision",
                        "email": "client2"
                    },
                    ... // more clients
                ],
                "decryption": "none"
            },
            "streamSettings": {
                "network": "tcp",
                "security": "reality",
                "realitySettings": { ... }
            }
        },
        {
            "listen": "127.0.0.1",
            "port": 10085,
            "protocol": "dokodemo-door",
            "settings": {
                "address": "127.0.0.1"
            },
            "tag": "api"
        }
    ],
    "outbounds": [ ... ],
    "stats": {},
    "api": {
        "tag": "api",
        "services": [
            "StatsService"
        ]
    },
    "policy": {
        "levels": {
            "0": {
                "statsUserUplink": true,
                "statsUserDownlink": true
            }
        }
    }
}
```

This will give you statistics available on port 10085 (refer to https://xtls.github.io/en/config/stats.html for further details and examples).
```bash
$ xray api statsquery -s 127.0.0.1:10085 -pattern client
{
    "stat": [
        {
            "name": "user>>>client1>>>traffic>>>downlink",
            "value": 123
        },
        {
            "name": "user>>>client2>>>traffic>>>downlink",
            "value": 456
        },
        {
            "name": "user>>>client1>>>traffic>>>uplink",
            "value": 789
        },
        {
            "name": "user>>>client2>>>traffic>>>uplink",
            "value": 111
        }
    ]
}
```
## Usage
Now you can run
```bash
export RUST_LOG=debug # optional, for other levels refer to env_logger crate documentation
cargo run http://127.0.0.1:10085 127.0.0.1:9185
```

Now the port 9185 will return data in Prometheus format:
```bash
$ curl 127.0.0.1:9185/metrics 2>&1 | grep xray
# HELP xray_downlink_bytes_total Xray downlink traffic in bytes
# TYPE xray_downlink_bytes_total counter
xray_downlink_bytes_total{user="client1"} 123
xray_downlink_bytes_total{user="client2"} 456
# HELP xray_uplink_bytes_total Xray uplink traffic in bytes
# TYPE xray_uplink_bytes_total counter
xray_uplink_bytes_total{user="client1"} 789
xray_uplink_bytes_total{user="client2"} 111
```

Now it could be integrated with anything which supports Prometheus (like Grafana).

## Implementation
Based on [tonic](https://docs.rs/tonic/latest/tonic/) and [prost](https://docs.rs/prost/latest/prost/) crates to query gRPC Xray endpoint and [prometheus_exporter](https://docs.rs/prometheus_exporter/latest/prometheus_exporter/). Connection pooling is handled with [bb8](https://docs.rs/bb8/latest/bb8/).

