mod xray {
    tonic::include_proto!("xray.app.stats.command");
}

use bb8::{ManageConnection, Pool};
use clap::Parser;
use prometheus_exporter::{self, prometheus::register_counter_vec};
use regex::Regex;
use tokio::runtime::Runtime;
use tonic::transport::Channel;
use xray::stats_service_client::StatsServiceClient;
use xray::{QueryStatsRequest, Stat};

#[derive(Parser)]
#[command(
    name = "Xray metrics exporter",
    about = "Exports stats provided by Xray in Prometheus format"
)]
struct Cli {
    xray_stats_endpoint: String,
    binding_address: String,
}

struct GrpcClientManager {
    uri: String,
}

impl ManageConnection for GrpcClientManager {
    type Connection = StatsServiceClient<Channel>;
    type Error = tonic::transport::Error;

    async fn connect(
        &self,
    ) -> Result<<Self as ManageConnection>::Connection, <Self as ManageConnection>::Error> {
        let uri = self.uri.clone();
        StatsServiceClient::connect(uri).await
    }

    async fn is_valid(
        &self,
        _: &mut <Self as ManageConnection>::Connection,
    ) -> Result<(), <Self as ManageConnection>::Error> {
        Ok(())
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let Cli {
        xray_stats_endpoint,
        binding_address,
    } = Cli::parse();

    let uplink_counter = register_counter_vec!(
        "xray_uplink_bytes_total",
        "Xray uplink traffic in bytes",
        &["user"]
    )?;

    let downlink_counter = register_counter_vec!(
        "xray_downlink_bytes_total",
        "Xray downlink traffic in bytes",
        &["user"]
    )?;

    let regex = Regex::new(r"^user>>>([^>]+)>>>traffic>>>([^>]+)$")?;

    let runtime = Runtime::new()?;

    let pool = runtime.block_on(Pool::builder().build(GrpcClientManager {
        uri: xray_stats_endpoint,
    }))?;

    let exporter = prometheus_exporter::start(binding_address.parse()?)?;

    loop {
        let _guard = exporter.wait_request();

        let request = tonic::Request::new(QueryStatsRequest {
            pattern: String::from("user>>>"),
            reset: false,
        });

        match runtime.block_on(pool.get()) {
            Ok(mut client) => {
                log::debug!("Querying stats with request {:?}", request);
                match runtime.block_on(client.query_stats(request)) {
                    Ok(response) => {
                        log::debug!("Stats response {:?}", response);
                        for Stat { name, value } in response.into_inner().stat {
                            if let Some(captures) = regex.captures(&name) {
                                if let Some(user) = captures.get(1).map(|m| m.as_str()) {
                                    match captures.get(2).map(|m| m.as_str()) {
                                        Some("uplink") => {
                                            let counter = uplink_counter.with_label_values(&[user]);
                                            counter.inc_by((value as f64) - counter.get());
                                        }
                                        Some("downlink") => {
                                            let counter =
                                                downlink_counter.with_label_values(&[user]);
                                            counter.inc_by((value as f64) - counter.get());
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Error querying stats {:?}", e)
                    }
                }
            }
            Err(e) => {
                log::warn!("Error connecting to gRPC {:?}", e)
            }
        }
    }
}
