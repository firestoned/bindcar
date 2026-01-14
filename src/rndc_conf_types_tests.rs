use super::*;

#[test]
fn test_key_block_serialization() {
    let key = KeyBlock::new(
        "rndc-key".to_string(),
        "hmac-sha256".to_string(),
        "dGVzdC1zZWNyZXQ=".to_string(),
    );

    let serialized = key.to_conf_block();
    assert!(serialized.contains("algorithm hmac-sha256;"));
    assert!(serialized.contains("secret \"dGVzdC1zZWNyZXQ=\";"));
}

#[test]
fn test_server_block_serialization() {
    let mut server = ServerBlock::new(ServerAddress::Hostname("localhost".to_string()));
    server.key = Some("rndc-key".to_string());
    server.port = Some(953);

    let serialized = server.to_conf_block();
    assert!(serialized.contains("key \"rndc-key\";"));
    assert!(serialized.contains("port 953;"));
}

#[test]
fn test_server_block_empty() {
    let server = ServerBlock::new(ServerAddress::Hostname("localhost".to_string()));
    let serialized = server.to_conf_block();
    assert_eq!(serialized, "{ };");
}

#[test]
fn test_options_block_serialization() {
    let mut options = OptionsBlock::new();
    options.default_server = Some("localhost".to_string());
    options.default_key = Some("rndc-key".to_string());
    options.default_port = Some(953);

    let serialized = options.to_conf_block();
    assert!(serialized.contains("default-server localhost;"));
    assert!(serialized.contains("default-key \"rndc-key\";"));
    assert!(serialized.contains("default-port 953;"));
}

#[test]
fn test_options_block_empty() {
    let options = OptionsBlock::new();
    assert!(options.is_empty());
    assert_eq!(options.to_conf_block(), "{ };");
}

#[test]
fn test_rndc_conf_file_serialization() {
    let mut conf = RndcConfFile::new();

    conf.keys.insert(
        "rndc-key".to_string(),
        KeyBlock::new(
            "rndc-key".to_string(),
            "hmac-sha256".to_string(),
            "dGVzdC1zZWNyZXQ=".to_string(),
        ),
    );

    conf.options.default_key = Some("rndc-key".to_string());
    conf.options.default_server = Some("localhost".to_string());

    let serialized = conf.to_conf_file();
    assert!(serialized.contains("key \"rndc-key\""));
    assert!(serialized.contains("algorithm hmac-sha256;"));
    assert!(serialized.contains("default-key \"rndc-key\";"));
    assert!(serialized.contains("default-server localhost;"));
}

#[test]
fn test_get_default_key() {
    let mut conf = RndcConfFile::new();

    conf.keys.insert(
        "rndc-key".to_string(),
        KeyBlock::new(
            "rndc-key".to_string(),
            "hmac-sha256".to_string(),
            "dGVzdC1zZWNyZXQ=".to_string(),
        ),
    );

    conf.options.default_key = Some("rndc-key".to_string());

    let key = conf.get_default_key().unwrap();
    assert_eq!(key.name, "rndc-key");
    assert_eq!(key.algorithm, "hmac-sha256");
}

#[test]
fn test_get_default_server() {
    let mut conf = RndcConfFile::new();
    conf.options.default_server = Some("localhost".to_string());

    assert_eq!(conf.get_default_server(), Some("localhost".to_string()));
}

#[test]
fn test_server_address_parse() {
    let ip_addr = ServerAddress::parse("127.0.0.1");
    assert!(matches!(ip_addr, ServerAddress::IpAddr(_)));

    let hostname = ServerAddress::parse("localhost");
    assert!(matches!(hostname, ServerAddress::Hostname(_)));
}

#[test]
fn test_server_address_display() {
    let hostname = ServerAddress::Hostname("localhost".to_string());
    assert_eq!(format!("{}", hostname), "localhost");

    let ip = ServerAddress::IpAddr("127.0.0.1".parse().unwrap());
    assert_eq!(format!("{}", ip), "127.0.0.1");
}

#[test]
fn test_include_serialization() {
    let mut conf = RndcConfFile::new();
    conf.includes.push(PathBuf::from("/etc/bind/rndc.key"));

    let serialized = conf.to_conf_file();
    assert!(serialized.contains("include \"/etc/bind/rndc.key\";"));
}

#[test]
fn test_server_with_addresses() {
    let mut server = ServerBlock::new(ServerAddress::Hostname("localhost".to_string()));
    server.addresses = Some(vec![
        "192.168.1.1".parse().unwrap(),
        "192.168.1.2".parse().unwrap(),
    ]);

    let serialized = server.to_conf_block();
    assert!(serialized.contains("addresses {"));
    assert!(serialized.contains("192.168.1.1;"));
    assert!(serialized.contains("192.168.1.2;"));
}
