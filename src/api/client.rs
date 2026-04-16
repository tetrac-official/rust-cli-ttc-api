use crate::config::{ApiConfig, AppConfig};
use crate::error::{Result, TtcError};
use crate::models::*;
use reqwest::Response;
use serde::de::DeserializeOwned;
use std::time::Duration;
use tracing::{debug, info, instrument};

const USER_AGENT: &str = concat!("skill-trading/", env!("CARGO_PKG_VERSION"), " (Rust)",);

#[derive(Clone)]
pub struct Client {
    inner: reqwest::Client,
    api_config: ApiConfig,
    api_key: String,
    public_key: String,
}

impl Client {
    pub fn new(config: &AppConfig) -> Result<Self> {
        let api_key = config
            .api_key
            .clone()
            .ok_or_else(|| TtcError::MissingConfig("ttc-auth-token".to_string()))?;
        let public_key = config
            .public_key
            .clone()
            .ok_or_else(|| TtcError::MissingConfig("ttc-public-key".to_string()))?;

        let inner = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.api.timeout))
            .user_agent(USER_AGENT)
            .build()
            .map_err(TtcError::from)?;

        Ok(Self {
            inner,
            api_config: config.api.clone(),
            api_key,
            public_key,
        })
    }

    /// Build headers for TTC API
    fn build_headers(&self) -> Result<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();

        headers.insert(
            "ttc-auth-token",
            self.api_key
                .parse()
                .map_err(|_| TtcError::Config("Invalid characters in ttc-auth-token".into()))?,
        );
        headers.insert(
            "ttc-public-key",
            self.public_key
                .parse()
                .map_err(|_| TtcError::Config("Invalid characters in ttc-public-key".into()))?,
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().expect("static header value"),
        );

        Ok(headers)
    }

    /// Execute request with retry logic
    async fn execute_with_retry<T: DeserializeOwned>(
        &self,
        request: reqwest::Request,
    ) -> Result<ApiResponse<T>> {
        let max_retries = self.api_config.max_retries;
        let mut attempt = 0;

        loop {
            attempt += 1;

            let request = request
                .try_clone()
                .ok_or_else(|| TtcError::Config("Failed to clone request for retry".to_string()))?;

            match self.inner.execute(request).await {
                Ok(response) => match self.handle_response(response).await {
                    Ok(result) => return Ok(result),
                    Err(e) if e.is_retryable() && attempt <= max_retries => {
                        let delay =
                            Duration::from_millis(self.api_config.retry_delay_ms * attempt as u64);
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    Err(e) => return Err(e),
                },
                Err(e) if attempt <= max_retries => {
                    let delay =
                        Duration::from_millis(self.api_config.retry_delay_ms * attempt as u64);
                    tokio::time::sleep(delay).await;
                    continue;
                }
                Err(e) => return Err(TtcError::Request(e)),
            }
        }
    }

    /// Handle API response
    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: Response,
    ) -> Result<ApiResponse<T>> {
        let status = response.status();
        let url = response.url().clone();

        debug!("Response status: {} from {}", status, url);

        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(60);
            return Err(TtcError::RateLimited(retry_after));
        }

        let body = response.text().await?;

        if !status.is_success() {
            return Err(TtcError::Api {
                code: status.as_u16(),
                message: body,
            });
        }

        let api_response: ApiResponse<T> = serde_json::from_str(&body)?;

        if !api_response.success {
            return Err(TtcError::Api {
                code: api_response.code,
                message: api_response
                    .message
                    .unwrap_or_else(|| "Unknown error".to_string()),
            });
        }

        Ok(api_response)
    }

    // ========================================================================
    // Order Methods
    // ========================================================================

    #[instrument(skip(self, params, credentials))]
    pub async fn place_limit_order(
        &self,
        exchange: &str,
        params: LimitOrderParams,
        credentials: ExchangeCredentials,
    ) -> Result<Order> {
        let request = ExchangeRequest {
            exchange_name: exchange.to_string(),
            method: "placeLimitOrder".to_string(),
            params,
            credentials,
        };

        info!("Placing limit order on {}", exchange);
        let response = self.post("/exchanges", &request).await?;
        Ok(response.data)
    }

    #[instrument(skip(self, params, credentials))]
    pub async fn place_market_order(
        &self,
        exchange: &str,
        params: MarketOrderParams,
        credentials: ExchangeCredentials,
    ) -> Result<Order> {
        let request = ExchangeRequest {
            exchange_name: exchange.to_string(),
            method: "placeMarketOrder".to_string(),
            params,
            credentials,
        };

        info!("Placing market order on {}", exchange);
        let response = self.post("/exchanges", &request).await?;
        Ok(response.data)
    }

    #[instrument(skip(self, params, credentials))]
    pub async fn place_stop_order(
        &self,
        exchange: &str,
        params: StopOrderParams,
        credentials: ExchangeCredentials,
    ) -> Result<Order> {
        let request = ExchangeRequest {
            exchange_name: exchange.to_string(),
            method: "placeStopOrder".to_string(),
            params,
            credentials,
        };

        info!("Placing stop order on {}", exchange);
        let response = self.post("/exchanges", &request).await?;
        Ok(response.data)
    }

    #[instrument(skip(self, credentials))]
    pub async fn get_orders(
        &self,
        exchange: &str,
        symbol: Option<&str>,
        credentials: ExchangeCredentials,
    ) -> Result<Vec<Order>> {
        let request = ExchangeRequest {
            exchange_name: exchange.to_string(),
            method: "getOrders".to_string(),
            params: symbol.map(String::from),
            credentials,
        };

        let response = self.post("/exchanges", &request).await?;
        Ok(response.data)
    }

    #[instrument(skip(self, params, credentials))]
    pub async fn cancel_order(
        &self,
        exchange: &str,
        params: CancelOrderParams,
        credentials: ExchangeCredentials,
    ) -> Result<bool> {
        let request = ExchangeRequest {
            exchange_name: exchange.to_string(),
            method: "cancelOrder".to_string(),
            params,
            credentials,
        };

        info!("Cancelling order on {}", exchange);
        self.post::<_, serde_json::Value>("/exchanges", &request)
            .await?;
        Ok(true)
    }

    #[instrument(skip(self, credentials))]
    pub async fn cancel_all_orders(
        &self,
        exchange: &str,
        symbol: Option<&str>,
        credentials: ExchangeCredentials,
    ) -> Result<CancelAllResult> {
        let request = ExchangeRequest {
            exchange_name: exchange.to_string(),
            method: "cancelAllOrders".to_string(),
            params: symbol.map(String::from),
            credentials,
        };

        info!("Cancelling all orders on {}", exchange);
        let response = self.post("/exchanges", &request).await?;
        Ok(response.data)
    }

    // ========================================================================
    // Position Methods
    // ========================================================================

    #[instrument(skip(self, credentials))]
    pub async fn get_positions(
        &self,
        exchange: &str,
        symbol: Option<&str>,
        credentials: ExchangeCredentials,
    ) -> Result<Vec<Position>> {
        let params = GetPositionsParams {
            symbol: symbol.map(String::from),
        };

        let request = ExchangeRequest {
            exchange_name: exchange.to_string(),
            method: "getPositions".to_string(),
            params,
            credentials,
        };

        let response = self.post("/exchanges", &request).await?;
        Ok(response.data)
    }

    #[instrument(skip(self, params, credentials))]
    pub async fn close_position(
        &self,
        exchange: &str,
        params: ClosePositionParams,
        credentials: ExchangeCredentials,
    ) -> Result<Order> {
        let request = ExchangeRequest {
            exchange_name: exchange.to_string(),
            method: "closePosition".to_string(),
            params,
            credentials,
        };

        info!("Closing position on {}", exchange);
        let response = self.post("/exchanges", &request).await?;
        Ok(response.data)
    }

    // ========================================================================
    // Account Methods
    // ========================================================================

    #[instrument(skip(self, credentials))]
    pub async fn get_balance(
        &self,
        exchange: &str,
        credentials: ExchangeCredentials,
    ) -> Result<Vec<Balance>> {
        let request = ExchangeRequest {
            exchange_name: exchange.to_string(),
            method: "getBalance".to_string(),
            params: GetBalanceParams { asset: None },
            credentials,
        };

        let response = self.post("/exchanges", &request).await?;
        Ok(response.data)
    }

    #[instrument(skip(self, params, credentials))]
    pub async fn set_leverage(
        &self,
        exchange: &str,
        params: SetLeverageParams,
        credentials: ExchangeCredentials,
    ) -> Result<LeverageResult> {
        let request = ExchangeRequest {
            exchange_name: exchange.to_string(),
            method: "setLeverage".to_string(),
            params,
            credentials,
        };

        info!("Setting leverage on {}", exchange);
        let response = self.post("/exchanges", &request).await?;
        Ok(response.data)
    }

    #[instrument(skip(self, params, credentials))]
    pub async fn set_margin_mode(
        &self,
        exchange: &str,
        params: SetMarginModeParams,
        credentials: ExchangeCredentials,
    ) -> Result<MarginModeResult> {
        let request = ExchangeRequest {
            exchange_name: exchange.to_string(),
            method: "setMarginMode".to_string(),
            params,
            credentials,
        };

        info!("Setting margin mode on {}", exchange);
        let response = self.post("/exchanges", &request).await?;
        Ok(response.data)
    }

    #[instrument(skip(self, credentials))]
    pub async fn set_hedge_mode(
        &self,
        exchange: &str,
        enabled: bool,
        credentials: ExchangeCredentials,
    ) -> Result<HedgeModeResult> {
        let request = ExchangeRequest {
            exchange_name: exchange.to_string(),
            method: "setHedgeMode".to_string(),
            params: serde_json::json!({ "hedgeMode": enabled }),
            credentials,
        };

        info!("Setting hedge mode on {}", exchange);
        let response = self.post("/exchanges", &request).await?;
        Ok(response.data)
    }

    // ========================================================================
    // Market Data Methods
    // ========================================================================

    #[instrument(skip(self, credentials))]
    pub async fn get_tickers(
        &self,
        exchange: &str,
        params: GetTickersParams,
        credentials: ExchangeCredentials,
    ) -> Result<Vec<Ticker>> {
        let request = ExchangeRequest {
            exchange_name: exchange.to_string(),
            method: "getTickers".to_string(),
            params,
            credentials,
        };

        let response = self.post("/exchanges", &request).await?;
        Ok(response.data)
    }

    #[instrument(skip(self, credentials))]
    pub async fn get_best_bid_ask(
        &self,
        exchange: &str,
        params: GetBestBidAskParams,
        credentials: ExchangeCredentials,
    ) -> Result<BestBidAsk> {
        // API wraps params in {symbol: params}, so send just the symbol string
        let request = ExchangeRequest {
            exchange_name: exchange.to_string(),
            method: "getBestBidAsk".to_string(),
            params: params.symbol,
            credentials,
        };

        let response = self.post("/exchanges", &request).await?;
        Ok(response.data)
    }

    // ========================================================================
    // HTTP Helper Methods
    // ========================================================================

    async fn post<T: serde::Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<ApiResponse<R>> {
        let url = format!("{}{}", self.api_config.base_url, path);
        let headers = self.build_headers()?;

        let request = self
            .inner
            .post(&url)
            .headers(headers)
            .json(body)
            .build()
            .map_err(TtcError::from)?;

        debug!("POST {}", url);

        self.execute_with_retry(request).await
    }

    async fn get<R: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<ApiResponse<R>> {
        let url = format!("{}{}", self.api_config.base_url, path);
        let headers = self.build_headers()?;

        let request = self
            .inner
            .get(&url)
            .headers(headers)
            .query(params)
            .build()
            .map_err(TtcError::from)?;

        debug!("GET {}", url);

        self.execute_with_retry(request).await
    }

    // ========================================================================
    // Market Data Methods (TTC Box direct endpoints)
    // ========================================================================

    #[instrument(skip(self))]
    #[allow(clippy::too_many_arguments)]
    pub async fn get_hybrid_tickers(
        &self,
        market_type: Option<&str>,
        exchange: Option<&str>,
        symbol: Option<&str>,
        min_volume: Option<f64>,
        min_price: Option<f64>,
        max_price: Option<f64>,
        up: Option<f64>,
        down: Option<f64>,
    ) -> Result<HybridTickersData> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(t) = market_type {
            params.push(("type", t.to_string()));
        }
        if let Some(e) = exchange {
            params.push(("exchange", e.to_string()));
        }
        if let Some(s) = symbol {
            params.push(("symbol", s.to_string()));
        }
        if let Some(v) = min_volume {
            params.push(("minimumVolume", v.to_string()));
        }
        if let Some(p) = min_price {
            params.push(("minimumPrice", p.to_string()));
        }
        if let Some(p) = max_price {
            params.push(("maximumPrice", p.to_string()));
        }
        if let Some(u) = up {
            params.push(("up", u.to_string()));
        }
        if let Some(d) = down {
            params.push(("down", d.to_string()));
        }

        let query: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let response = self
            .get::<HybridTickersData>("/markets/hybrid-tickers", &query)
            .await?;
        Ok(response.data)
    }

    #[instrument(skip(self))]
    pub async fn get_funding_rates(&self, symbol: Option<&str>) -> Result<Vec<FundingRate>> {
        let params: Vec<(&str, &str)> = symbol.map(|s| vec![("symbol", s)]).unwrap_or_default();
        let response = self
            .get::<Vec<FundingRate>>("/markets/funding-rates", &params)
            .await?;
        Ok(response.data)
    }

    #[instrument(skip(self))]
    pub async fn get_open_interest(&self, symbol: Option<&str>) -> Result<Vec<OpenInterestItem>> {
        let body = symbol
            .map(|s| serde_json::json!({ "symbol": s }))
            .unwrap_or_else(|| serde_json::json!({}));
        let response = self
            .post::<_, Vec<OpenInterestItem>>("/markets/open-interest", &body)
            .await?;
        Ok(response.data)
    }

    #[instrument(skip(self))]
    pub async fn get_volume_snapshot(&self) -> Result<Vec<VolumeSnapshotExchange>> {
        let response = self
            .get::<Vec<VolumeSnapshotExchange>>("/markets/volume-snapshot", &[])
            .await?;
        Ok(response.data)
    }

    #[instrument(skip(self))]
    pub async fn get_scanner(
        &self,
        symbol: &str,
        timeframe: Option<&str>,
        bars: Option<u32>,
        swing_strength: Option<u32>,
    ) -> Result<ScannerResult> {
        let mut params: Vec<(&str, String)> = vec![("symbol", symbol.to_string())];
        if let Some(tf) = timeframe {
            params.push(("timeframe", tf.to_string()));
        }
        if let Some(b) = bars {
            params.push(("bars", b.to_string()));
        }
        if let Some(s) = swing_strength {
            params.push(("swingStrength", s.to_string()));
        }

        let url = format!("{}/markets/ttc-scanner", self.api_config.base_url);
        let headers = self.build_headers()?;
        let request = self
            .inner
            .get(&url)
            .headers(headers)
            .query(&params)
            .build()
            .map_err(TtcError::from)?;
        debug!("GET {}", url);
        let response: ApiResponse<ScannerResult> = self.execute_with_retry(request).await?;
        Ok(response.data)
    }
}
