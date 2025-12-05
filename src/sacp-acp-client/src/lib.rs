use extension_trait::extension_trait;
use sacp::JrConnectionCx;

#[extension_trait]
pub impl AcpClient for JrConnectionCx {}
