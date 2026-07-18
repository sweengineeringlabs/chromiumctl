/// Builder for [`CdpClient`] — configure launch options before connecting.
///
/// [`CdpClient`]: super::CdpClient
pub struct CdpClientBuilder {
    url:        String,
    chrome_bin: Option<String>,
    port:       Option<u16>,
}

impl CdpClientBuilder {
    /// Start building a client that will open `url` after launch.
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into(), chrome_bin: None, port: None }
    }

    /// Override the browser binary path instead of using auto-discovery.
    pub fn chrome_bin(mut self, path: impl Into<String>) -> Self {
        self.chrome_bin = Some(path.into());
        self
    }

    /// Fix the remote-debugging port instead of using the auto-assigned value.
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Launch the browser and return a connected [`CdpClient`].
    pub fn launch(self) -> Result<super::CdpClient, String> {
        if let Some(bin) = self.chrome_bin {
            std::env::set_var("CHROME_PATH", bin);
        }
        super::CdpClient::launch_on_port(&self.url, self.port)
    }
}
