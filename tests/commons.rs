use std::net::{SocketAddr, TcpListener};

use growthbook_rust::client::GrowthBookClient;
use rand::Rng;
use reqwest::StatusCode;
use serde_json::{json, Value};
use test_context::AsyncTestContext;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[allow(dead_code)]
pub struct TestContext {
    pub mock_server: MockServer,
    pub growthbook: GrowthBookClient,
}

impl AsyncTestContext for TestContext {
    async fn setup() -> TestContext {
        let mock_server = create_mock_server().await;
        let gb_sdk = Uuid::now_v7();

        all_cases(&mock_server, gb_sdk).await;

        let growthbook = GrowthBookClient::new(&mock_server.uri(), gb_sdk.to_string().as_str(), None, None)
            .await
            .expect("Failed to create growthbook gateway");

        TestContext { mock_server, growthbook }
    }
}

pub async fn all_cases(
    mock_server: &MockServer,
    sdk: Uuid,
) {
    let body = json!({
        "status": 200,
        "features": {
            "flag": {
                "defaultValue": false,
                "rules": [
                    {
                        "condition": {
                            "any-id": "018fcf11-bb67-7789-8d10-fcbb7de4ff7b",
                            "any-key": "018fcf64-1827-709a-a8ae-7d206aafb5e2"
                        },
                        "force": true
                    }
                ]
            },
            "fixed-value": {
                "defaultValue": "018fcf11-bb67-7789-8d10-fcbb7de4ff7b"
            },
            "lte-flag": {
                "defaultValue": false,
                "rules": [
                    {
                        "condition": {
                            "version": {
                                "$lte": "1.2.3"
                            }
                        },
                        "force": true,
                    }
                ]
            },
            "object-flag": {
                "defaultValue": {
                    "a": "potato",
                    "b": "tomato"
                }
            },
            "elem-match-eq": {
                "defaultValue": false,
                "rules": [
                    {
                        "condition": {
                            "any-data": {
                                "$elemMatch": {
                                    "$eq": 3
                                }
                            }
                        },
                        "force": true,
                    }
                ]
            },
            "gt-rule": {
                "defaultValue": false,
                "rules": [
                    {
                        "condition": {
                            "version": {
                                "$gt": "1.2.3"
                            }
                        },
                        "force": true,
                    }
                ]
            },
            "gte-rule": {
                "defaultValue": false,
                "rules": [
                    {
                        "condition": {
                            "version": {
                                "$gte": "1.2.3"
                            }
                        },
                        "force": true,
                    }
                ]
            },
            "lt-rule": {
                "defaultValue": false,
                "rules": [
                    {
                        "condition": {
                            "version": {
                                "$lt": "1.2.3"
                            }
                        },
                        "force": true,
                    }
                ]
            },
            "not-elem-match-eq": {
                "defaultValue": false,
                "rules": [
                    {
                        "condition": {
                            "any-data": {
                                "$not": {
                                    "$elemMatch": {
                                        "$eq": "3"
                                    }
                                }
                            }
                        },
                        "force": true,
                    }
                ]
            },
            "regex-rule": {
                "defaultValue": false,
                "rules": [
                    {
                        "condition": {
                            "version": {
                                "$regex": "^[3-9]\\d*(\\.\\d+)*$"
                            }
                        },
                        "force": true,
                    }
                ]
            },
            "experiment-rule-condition-flag": {
                "defaultValue": false,
                "rules": [
                    {
                        "coverage": 1.0,
                        "hashAttribute": "any-id",
                        "seed": "any-seed",
                        "hashVersion": 2,
                        "variations": [
                            false,
                            true,
                            true
                        ],
                        "weights": [
                            0.3334,
                            0.3333,
                            0.3333
                        ],
                        "key": "any-experiment-key",
                        "meta": [
                            {
                                "key": "0"
                            },
                            {
                                "key": "1"
                            },
                            {
                                "key": "2"
                            }
                        ],
                        "phase": "1"
                    }
                ]
            },
        "experiment-rule-condition-zero-coverage-flag": {
                "defaultValue": false,
                "rules": [
                    {
                        "coverage": 0.0,
                        "hashAttribute": "any-id",
                        "seed": "any-seed",
                        "hashVersion": 2,
                        "variations": [
                            false,
                            true,
                            true
                        ],
                        "weights": [
                            0.3334,
                            0.3333,
                            0.3333
                        ],
                        "key": "any-experiment-key",
                        "meta": [
                            {
                                "key": "0"
                            },
                            {
                                "key": "1"
                            },
                            {
                                "key": "2"
                            }
                        ],
                        "phase": "1"
                    }
                ]
            },
        "experiment-rule-condition-ninety-coverage-flag": {
                "defaultValue": false,
                "rules": [
                    {
                        "coverage": 0.9,
                        "hashAttribute": "any-id",
                        "seed": "any-seed",
                        "hashVersion": 2,
                        "variations": [
                            false,
                            true,
                            true
                        ],
                        "weights": [
                            0.3334,
                            0.3333,
                            0.3333
                        ],
                        "key": "any-experiment-key",
                        "meta": [
                            {
                                "key": "0"
                            },
                            {
                                "key": "1"
                            },
                            {
                                "key": "2"
                            }
                        ],
                        "phase": "1"
                    }
                ]
            },
            "simple-flag": {
                "defaultValue": true
            },
            "simple-flag-disabled": {
                "defaultValue": false
            },
            "simple-rule-conditio": {
                "defaultValue": false,
                "rules": [
                    {
                        "condition": {
                            "any-id": "018fcf11-bb67-7789-8d10-fcbb7de4ff7b"
                        },
                        "force": true
                    }
                ]
            },
            "rollout-flag-condition-by-attribute": {
                "defaultValue": false,
                "rules": [
                    {
                        "force": true,
                        "coverage": 0.5,
                        "hashAttribute": "any-id"
                    }
                ]
            },
            "rollout-zero-percentage-flag-condition-by-attribute": {
                "defaultValue": false,
                "rules": [
                    {
                        "force": true,
                        "coverage": 0.0,
                        "hashAttribute": "any-id"
                    }
                ]
            },
            "rollout-one-hundred-percentage-flag-condition-by-attribute": {
                "defaultValue": false,
                "rules": [
                    {
                        "force": true,
                        "coverage": 1.0,
                        "hashAttribute": "any-id"
                    }
                ]
            }
        },
        "dateUpdated": "2024-05-29T18:43:22.153Z"
    });
    mock(mock_server, sdk, body, 1, StatusCode::OK).await;
}

async fn mock(
    mock_server: &MockServer,
    sdk: Uuid,
    body: Value,
    times: u64,
    status_code: StatusCode,
) {
    let response = if status_code.is_success() {
        ResponseTemplate::new(status_code.as_u16()).set_body_string(body.to_string())
    } else {
        ResponseTemplate::new(status_code.as_u16())
    };

    Mock::given(method("GET"))
        .and(path(format!("/api/features/{sdk}")))
        .respond_with(response)
        .expect(times)
        .mount(mock_server)
        .await;
}

pub async fn create_mock_server() -> MockServer {
    for _ in 1..10 {
        // try to start mock server in a random port 10 times
        let port = rand::thread_rng().gen_range(51000..54000);
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        if let Ok(listener) = TcpListener::bind(addr) {
            let mock_server = MockServer::builder().listener(listener).start().await;
            return mock_server;
        }
    }
    panic!("Failed to create mock server");
}
