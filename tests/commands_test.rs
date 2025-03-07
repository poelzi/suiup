// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use suiup::commands::{parse_component_with_version, BinaryName, CommandMetadata};

    #[test]
    fn test_parse_component_with_version() -> Result<(), anyhow::Error> {
        let result = parse_component_with_version("sui")?;
        let expected = CommandMetadata {
            name: BinaryName::Sui,
            network: "testnet".to_string(),
            version: None,
        };
        assert_eq!(expected, result);

        let result = parse_component_with_version("sui testnet-v1.39.3")?;
        let expected = CommandMetadata {
            name: BinaryName::Sui,
            network: "testnet".to_string(),
            version: Some("v1.39.3".to_string()),
        };
        assert_eq!(expected, result,);

        let result = parse_component_with_version("sui-bridge devnet")?;
        let expected = CommandMetadata {
            name: BinaryName::SuiBridge,
            network: "devnet".to_string(),
            version: None,
        };
        assert_eq!(expected, result);
        let result = parse_component_with_version("walrus")?;
        let expected = CommandMetadata {
            name: BinaryName::Walrus,
            network: "testnet".to_string(),
            version: None,
        };
        assert_eq!(expected, result);

        let result = parse_component_with_version("mvr")?;
        let expected = CommandMetadata {
            name: BinaryName::Mvr,
            network: "testnet".to_string(),
            version: None,
        };
        assert_eq!(expected, result);

        let result = parse_component_with_version("random");
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid component name: random".to_string()
        );

        Ok(())
    }

    #[test]
    fn test_sui_component_display() {
        assert_eq!(BinaryName::Sui.to_string(), "sui");
        assert_eq!(BinaryName::SuiBridge.to_string(), "sui-bridge");
        assert_eq!(BinaryName::SuiFaucet.to_string(), "sui-faucet");
        assert_eq!(BinaryName::Mvr.to_string(), "mvr");
        assert_eq!(BinaryName::Walrus.to_string(), "walrus");
    }
}
