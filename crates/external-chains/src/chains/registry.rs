//! Universal Chain Registry - ALL EVM Chains
//!
//! YOLO MODE: Every EVM chain in existence. Copy-paste glory.

use sp_std::vec::Vec;

/// Chain metadata for any EVM-compatible network
#[derive(Debug, Clone)]
pub struct ChainInfo {
    pub chain_id: u64,
    pub name: &'static str,
    pub symbol: &'static str,
    pub rpc: &'static str,
    pub explorer: &'static str,
    pub is_l2: bool,
    pub block_time_ms: u64,
    pub confirmations: u32,
}

/// ALL THE CHAINS - Tier 1, 2, and 3
pub static ALL_CHAINS: &[ChainInfo] = &[
    // ═══════════════════════════════════════════════════════════════════
    // TIER 1 - MAJOR NETWORKS
    // ═══════════════════════════════════════════════════════════════════

    // Ethereum Mainnet
    ChainInfo {
        chain_id: 1,
        name: "Ethereum",
        symbol: "ETH",
        rpc: "https://eth.llamarpc.com",
        explorer: "https://etherscan.io",
        is_l2: false,
        block_time_ms: 12000,
        confirmations: 12,
    },
    // Optimism
    ChainInfo {
        chain_id: 10,
        name: "Optimism",
        symbol: "ETH",
        rpc: "https://mainnet.optimism.io",
        explorer: "https://optimistic.etherscan.io",
        is_l2: true,
        block_time_ms: 2000,
        confirmations: 1,
    },
    // Cronos
    ChainInfo {
        chain_id: 25,
        name: "Cronos",
        symbol: "CRO",
        rpc: "https://evm.cronos.org",
        explorer: "https://cronoscan.com",
        is_l2: false,
        block_time_ms: 6000,
        confirmations: 12,
    },
    // BNB Smart Chain (already implemented, keeping for registry)
    ChainInfo {
        chain_id: 56,
        name: "BNB Smart Chain",
        symbol: "BNB",
        rpc: "https://bsc-dataseed.binance.org",
        explorer: "https://bscscan.com",
        is_l2: false,
        block_time_ms: 3000,
        confirmations: 15,
    },
    // Polygon (already implemented)
    ChainInfo {
        chain_id: 137,
        name: "Polygon",
        symbol: "POL",
        rpc: "https://polygon-rpc.com",
        explorer: "https://polygonscan.com",
        is_l2: false,
        block_time_ms: 2000,
        confirmations: 128,
    },
    // Fantom Opera
    ChainInfo {
        chain_id: 250,
        name: "Fantom Opera",
        symbol: "FTM",
        rpc: "https://rpc.ftm.tools",
        explorer: "https://ftmscan.com",
        is_l2: false,
        block_time_ms: 1000,
        confirmations: 1,
    },
    // Klaytn
    ChainInfo {
        chain_id: 8217,
        name: "Klaytn",
        symbol: "KLAY",
        rpc: "https://public-en-cypress.klaytn.net",
        explorer: "https://scope.klaytn.com",
        is_l2: false,
        block_time_ms: 1000,
        confirmations: 1,
    },
    // Base (already implemented)
    ChainInfo {
        chain_id: 8453,
        name: "Base",
        symbol: "ETH",
        rpc: "https://mainnet.base.org",
        explorer: "https://basescan.org",
        is_l2: true,
        block_time_ms: 2000,
        confirmations: 1,
    },
    // Arbitrum One (already implemented)
    ChainInfo {
        chain_id: 42161,
        name: "Arbitrum One",
        symbol: "ETH",
        rpc: "https://arb1.arbitrum.io/rpc",
        explorer: "https://arbiscan.io",
        is_l2: true,
        block_time_ms: 250,
        confirmations: 1,
    },
    // Avalanche C-Chain (already implemented)
    ChainInfo {
        chain_id: 43114,
        name: "Avalanche C-Chain",
        symbol: "AVAX",
        rpc: "https://api.avax.network/ext/bc/C/rpc",
        explorer: "https://snowtrace.io",
        is_l2: false,
        block_time_ms: 2000,
        confirmations: 1,
    },
    // ═══════════════════════════════════════════════════════════════════
    // TIER 2 - ESTABLISHED NETWORKS
    // ═══════════════════════════════════════════════════════════════════

    // Expanse
    ChainInfo {
        chain_id: 2,
        name: "Expanse",
        symbol: "EXP",
        rpc: "https://node.expanse.tech",
        explorer: "https://explorer.expanse.tech",
        is_l2: false,
        block_time_ms: 60000,
        confirmations: 12,
    },
    // ThaiChain
    ChainInfo {
        chain_id: 7,
        name: "ThaiChain",
        symbol: "TCH",
        rpc: "https://rpc.thaichain.org",
        explorer: "https://exp.thaichain.org",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Ubiq
    ChainInfo {
        chain_id: 8,
        name: "Ubiq",
        symbol: "UBQ",
        rpc: "https://rpc.octano.dev",
        explorer: "https://ubiqscan.io",
        is_l2: false,
        block_time_ms: 88000,
        confirmations: 12,
    },
    // Metadium
    ChainInfo {
        chain_id: 11,
        name: "Metadium",
        symbol: "META",
        rpc: "https://api.metadium.com/prod",
        explorer: "https://explorer.metadium.com",
        is_l2: false,
        block_time_ms: 1000,
        confirmations: 12,
    },
    // Flare
    ChainInfo {
        chain_id: 14,
        name: "Flare",
        symbol: "FLR",
        rpc: "https://flare-api.flare.network/ext/C/rpc",
        explorer: "https://flare-explorer.flare.network",
        is_l2: false,
        block_time_ms: 3000,
        confirmations: 1,
    },
    // Diode
    ChainInfo {
        chain_id: 15,
        name: "Diode",
        symbol: "DIODE",
        rpc: "https://prenet.diode.io:8443",
        explorer: "https://diode.io/prenet",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // ThaiChain 2.0
    ChainInfo {
        chain_id: 17,
        name: "ThaiChain 2.0",
        symbol: "TFI",
        rpc: "https://rpc.thaichain.io",
        explorer: "https://exp.thaichain.io",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Songbird
    ChainInfo {
        chain_id: 19,
        name: "Songbird",
        symbol: "SGB",
        rpc: "https://songbird-api.flare.network/ext/C/rpc",
        explorer: "https://songbird-explorer.flare.network",
        is_l2: false,
        block_time_ms: 3000,
        confirmations: 1,
    },
    // Elastos
    ChainInfo {
        chain_id: 20,
        name: "Elastos",
        symbol: "ELA",
        rpc: "https://api.elastos.io/eth",
        explorer: "https://esc.elastos.io",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // ELA-DID
    ChainInfo {
        chain_id: 22,
        name: "ELA-DID-Sidechain",
        symbol: "ELA",
        rpc: "https://api.elastos.io/did",
        explorer: "https://idchain.elastos.org",
        is_l2: true,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // KardiaChain
    ChainInfo {
        chain_id: 24,
        name: "KardiaChain",
        symbol: "KAI",
        rpc: "https://rpc.kardiachain.io",
        explorer: "https://explorer.kardiachain.io",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // ShibaChain
    ChainInfo {
        chain_id: 27,
        name: "ShibaChain",
        symbol: "SHIB",
        rpc: "https://rpc.shibachain.net",
        explorer: "https://exp.shibachain.net",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Genesis L1
    ChainInfo {
        chain_id: 29,
        name: "Genesis L1",
        symbol: "L1",
        rpc: "https://rpc.genesisL1.org",
        explorer: "https://explorer.genesisL1.org",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // RSK
    ChainInfo {
        chain_id: 30,
        name: "RSK Mainnet",
        symbol: "RBTC",
        rpc: "https://public-node.rsk.co",
        explorer: "https://explorer.rsk.co",
        is_l2: false,
        block_time_ms: 30000,
        confirmations: 12,
    },
    // GoodData
    ChainInfo {
        chain_id: 33,
        name: "GoodData",
        symbol: "GOO",
        rpc: "https://rpc.goodata.io",
        explorer: "https://explorer.goodata.io",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // TBWG
    ChainInfo {
        chain_id: 35,
        name: "TBWG Chain",
        symbol: "TBG",
        rpc: "https://rpc.tbwg.io",
        explorer: "https://explorer.tbwg.io",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Valorbit
    ChainInfo {
        chain_id: 38,
        name: "Valorbit",
        symbol: "VAL",
        rpc: "https://rpc.valorbit.com",
        explorer: "https://explorer.valorbit.com",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Telos
    ChainInfo {
        chain_id: 40,
        name: "Telos EVM",
        symbol: "TLOS",
        rpc: "https://mainnet.telos.net/evm",
        explorer: "https://teloscan.io",
        is_l2: false,
        block_time_ms: 500,
        confirmations: 1,
    },
    // Darwinia Crab
    ChainInfo {
        chain_id: 44,
        name: "Darwinia Crab",
        symbol: "CRAB",
        rpc: "https://crab-rpc.darwinia.network",
        explorer: "https://crab.subscan.io",
        is_l2: false,
        block_time_ms: 6000,
        confirmations: 12,
    },
    // XinFin XDC
    ChainInfo {
        chain_id: 50,
        name: "XinFin XDC",
        symbol: "XDC",
        rpc: "https://rpc.xinfin.network",
        explorer: "https://explorer.xinfin.network",
        is_l2: false,
        block_time_ms: 2000,
        confirmations: 12,
    },
    // CoinEx
    ChainInfo {
        chain_id: 52,
        name: "CoinEx Smart Chain",
        symbol: "CET",
        rpc: "https://rpc.coinex.net",
        explorer: "https://www.coinex.net",
        is_l2: false,
        block_time_ms: 3000,
        confirmations: 12,
    },
    // Zyx
    ChainInfo {
        chain_id: 55,
        name: "Zyx Mainnet",
        symbol: "ZYX",
        rpc: "https://rpc-1.zyx.network",
        explorer: "https://zyxscan.com",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Syscoin
    ChainInfo {
        chain_id: 57,
        name: "Syscoin",
        symbol: "SYS",
        rpc: "https://rpc.syscoin.org",
        explorer: "https://explorer.syscoin.org",
        is_l2: false,
        block_time_ms: 150000,
        confirmations: 6,
    },
    // Ontology
    ChainInfo {
        chain_id: 58,
        name: "Ontology",
        symbol: "ONG",
        rpc: "https://dappnode1.ont.io:10339",
        explorer: "https://explorer.ont.io",
        is_l2: false,
        block_time_ms: 1000,
        confirmations: 12,
    },
    // EOS EVM
    ChainInfo {
        chain_id: 59,
        name: "EOS EVM",
        symbol: "EOS",
        rpc: "https://api.evm.eosnetwork.com",
        explorer: "https://explorer.evm.eosnetwork.com",
        is_l2: false,
        block_time_ms: 500,
        confirmations: 1,
    },
    // GoChain
    ChainInfo {
        chain_id: 60,
        name: "GoChain",
        symbol: "GO",
        rpc: "https://rpc.gochain.io",
        explorer: "https://explorer.gochain.io",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // Ethereum Classic
    ChainInfo {
        chain_id: 61,
        name: "Ethereum Classic",
        symbol: "ETC",
        rpc: "https://etc.rivet.link",
        explorer: "https://blockscout.com/etc/mainnet",
        is_l2: false,
        block_time_ms: 13000,
        confirmations: 120,
    },
    // Ellaism
    ChainInfo {
        chain_id: 64,
        name: "Ellaism",
        symbol: "ELLA",
        rpc: "https://jsonrpc.ellaism.org",
        explorer: "https://explorer.ellaism.org",
        is_l2: false,
        block_time_ms: 14000,
        confirmations: 12,
    },
    // OKExChain
    ChainInfo {
        chain_id: 66,
        name: "OKXChain",
        symbol: "OKT",
        rpc: "https://exchainrpc.okex.org",
        explorer: "https://www.oklink.com/okc",
        is_l2: false,
        block_time_ms: 3000,
        confirmations: 12,
    },
    // SoterOne
    ChainInfo {
        chain_id: 68,
        name: "SoterOne",
        symbol: "SOTER",
        rpc: "https://rpc.soter.one",
        explorer: "https://explorer.soter.one",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // IDChain
    ChainInfo {
        chain_id: 74,
        name: "IDChain",
        symbol: "EIDI",
        rpc: "https://idchain.one/rpc",
        explorer: "https://explorer.idchain.one",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // Mix
    ChainInfo {
        chain_id: 76,
        name: "Mix",
        symbol: "MIX",
        rpc: "https://rpc2.mix-blockchain.org:8647",
        explorer: "https://blocks.mix-blockchain.org",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // POA Sokol
    ChainInfo {
        chain_id: 77,
        name: "POA Sokol",
        symbol: "SPOA",
        rpc: "https://sokol.poa.network",
        explorer: "https://blockscout.com/poa/sokol",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // PrimusChain
    ChainInfo {
        chain_id: 78,
        name: "PrimusChain",
        symbol: "PETH",
        rpc: "https://ethnode.primusmoney.com/mainnet",
        explorer: "https://explorer.primusmoney.com",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // GeneChain
    ChainInfo {
        chain_id: 80,
        name: "GeneChain",
        symbol: "RNA",
        rpc: "https://rpc.genechain.io",
        explorer: "https://scan.genechain.io",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Meter
    ChainInfo {
        chain_id: 82,
        name: "Meter",
        symbol: "MTR",
        rpc: "https://rpc.meter.io",
        explorer: "https://scan.meter.io",
        is_l2: false,
        block_time_ms: 2000,
        confirmations: 12,
    },
    // GateChain
    ChainInfo {
        chain_id: 86,
        name: "GateChain",
        symbol: "GT",
        rpc: "https://evm.gatenode.cc",
        explorer: "https://gatescan.org",
        is_l2: false,
        block_time_ms: 4000,
        confirmations: 12,
    },
    // Nova Network
    ChainInfo {
        chain_id: 87,
        name: "Nova Network",
        symbol: "SNT",
        rpc: "https://connect.novanetwork.io",
        explorer: "https://explorer.novanetwork.io",
        is_l2: false,
        block_time_ms: 3000,
        confirmations: 12,
    },
    // TomoChain
    ChainInfo {
        chain_id: 88,
        name: "TomoChain",
        symbol: "TOMO",
        rpc: "https://rpc.tomochain.com",
        explorer: "https://tomoscan.io",
        is_l2: false,
        block_time_ms: 2000,
        confirmations: 12,
    },
    // NEXT
    ChainInfo {
        chain_id: 96,
        name: "NEXT Smart Chain",
        symbol: "NEXT",
        rpc: "https://rpc.nextsmartchain.com",
        explorer: "https://explorer.nextsmartchain.com",
        is_l2: false,
        block_time_ms: 3000,
        confirmations: 12,
    },
    // POA Core
    ChainInfo {
        chain_id: 99,
        name: "POA Network Core",
        symbol: "POA",
        rpc: "https://core.poa.network",
        explorer: "https://blockscout.com/poa/core",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // Gnosis (xDai)
    ChainInfo {
        chain_id: 100,
        name: "Gnosis Chain",
        symbol: "xDAI",
        rpc: "https://rpc.gnosischain.com",
        explorer: "https://gnosisscan.io",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // EtherInc
    ChainInfo {
        chain_id: 101,
        name: "EtherInc",
        symbol: "ETI",
        rpc: "https://api.einc.io/jsonrpc/mainnet",
        explorer: "https://explorer.einc.io",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Velas
    ChainInfo {
        chain_id: 106,
        name: "Velas EVM",
        symbol: "VLX",
        rpc: "https://evmexplorer.velas.com/rpc",
        explorer: "https://evmexplorer.velas.com",
        is_l2: false,
        block_time_ms: 400,
        confirmations: 1,
    },
    // ThunderCore
    ChainInfo {
        chain_id: 108,
        name: "ThunderCore",
        symbol: "TT",
        rpc: "https://mainnet-rpc.thundercore.com",
        explorer: "https://viewblock.io/thundercore",
        is_l2: false,
        block_time_ms: 1000,
        confirmations: 12,
    },
    // EtherLite
    ChainInfo {
        chain_id: 111,
        name: "EtherLite",
        symbol: "ETL",
        rpc: "https://rpc.etherlite.org",
        explorer: "https://explorer.etherlite.org",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Fuse
    ChainInfo {
        chain_id: 122,
        name: "Fuse",
        symbol: "FUSE",
        rpc: "https://rpc.fuse.io",
        explorer: "https://explorer.fuse.io",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // Fuse Sparknet
    ChainInfo {
        chain_id: 123,
        name: "Fuse Sparknet",
        symbol: "SPARK",
        rpc: "https://rpc.fusespark.io",
        explorer: "https://explorer.fusespark.io",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // Decentralized Web
    ChainInfo {
        chain_id: 124,
        name: "Decentralized Web",
        symbol: "DWU",
        rpc: "https://decentralized-web.tech/dw_rpc.php",
        explorer: "https://decentralized-web.tech/dw_explorer",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // OYchain
    ChainInfo {
        chain_id: 126,
        name: "OYchain",
        symbol: "OY",
        rpc: "https://rpc.mainnet.oychain.io",
        explorer: "https://explorer.oychain.io",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Factory 127
    ChainInfo {
        chain_id: 127,
        name: "Factory 127",
        symbol: "FETH",
        rpc: "https://rpc.factory127.com",
        explorer: "https://explorer.factory127.com",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Huobi ECO
    ChainInfo {
        chain_id: 128,
        name: "Huobi ECO Chain",
        symbol: "HT",
        rpc: "https://http-mainnet.hecochain.com",
        explorer: "https://hecoinfo.com",
        is_l2: false,
        block_time_ms: 3000,
        confirmations: 12,
    },
    // DAX
    ChainInfo {
        chain_id: 142,
        name: "DAX CHAIN",
        symbol: "DAX",
        rpc: "https://rpc.prodax.io",
        explorer: "https://explorer.prodax.io",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Lightstreams
    ChainInfo {
        chain_id: 163,
        name: "Lightstreams",
        symbol: "PHT",
        rpc: "https://node.mainnet.lightstreams.io",
        explorer: "https://explorer.lightstreams.io",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // Seele
    ChainInfo {
        chain_id: 186,
        name: "Seele",
        symbol: "SEELE",
        rpc: "https://rpc.seelen.pro",
        explorer: "https://seelen.pro",
        is_l2: false,
        block_time_ms: 14000,
        confirmations: 12,
    },
    // BMC
    ChainInfo {
        chain_id: 188,
        name: "BMC Mainnet",
        symbol: "BTM",
        rpc: "https://mainnet.bmcchain.com",
        explorer: "https://bmc.blockmeta.com",
        is_l2: false,
        block_time_ms: 500,
        confirmations: 12,
    },
    // BitTorrent Chain
    ChainInfo {
        chain_id: 199,
        name: "BitTorrent Chain",
        symbol: "BTT",
        rpc: "https://rpc.bt.io",
        explorer: "https://bttcscan.com",
        is_l2: false,
        block_time_ms: 2000,
        confirmations: 12,
    },
    // Arbitrum on xDai
    ChainInfo {
        chain_id: 200,
        name: "Arbitrum on xDai",
        symbol: "xDAI",
        rpc: "https://arbitrum.xdaichain.com",
        explorer: "https://blockscout.com/xdai/arbitrum",
        is_l2: true,
        block_time_ms: 5000,
        confirmations: 1,
    },
    // opBNB
    ChainInfo {
        chain_id: 204,
        name: "opBNB",
        symbol: "BNB",
        rpc: "https://opbnb-mainnet-rpc.bnbchain.org",
        explorer: "https://opbnbscan.com",
        is_l2: true,
        block_time_ms: 1000,
        confirmations: 1,
    },
    // Freight Trust
    ChainInfo {
        chain_id: 211,
        name: "Freight Trust",
        symbol: "0xF",
        rpc: "https://13.57.207.168:3435",
        explorer: "https://explorer.freight.sh",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Permission
    ChainInfo {
        chain_id: 222,
        name: "Permission",
        symbol: "ASK",
        rpc: "https://blockchain-api-mainnet.permission.io/rpc",
        explorer: "https://explorer.permission.io",
        is_l2: false,
        block_time_ms: 6000,
        confirmations: 12,
    },
    // Energy Web
    ChainInfo {
        chain_id: 246,
        name: "Energy Web Chain",
        symbol: "EWT",
        rpc: "https://rpc.energyweb.org",
        explorer: "https://explorer.energyweb.org",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // Setheum
    ChainInfo {
        chain_id: 258,
        name: "Setheum",
        symbol: "SETM",
        rpc: "https://rpc.setheum.xyz",
        explorer: "https://explorer.setheum.xyz",
        is_l2: false,
        block_time_ms: 6000,
        confirmations: 12,
    },
    // SUR
    ChainInfo {
        chain_id: 262,
        name: "SUR Blockchain",
        symbol: "SRN",
        rpc: "https://sur.nilin.org",
        explorer: "https://explorer.surnet.org",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // High Performance Blockchain
    ChainInfo {
        chain_id: 269,
        name: "HPB",
        symbol: "HPB",
        rpc: "https://hpbnode.com",
        explorer: "https://hscan.org",
        is_l2: false,
        block_time_ms: 3000,
        confirmations: 12,
    },
    // Boba Network
    ChainInfo {
        chain_id: 288,
        name: "Boba Network",
        symbol: "ETH",
        rpc: "https://mainnet.boba.network",
        explorer: "https://bobascan.com",
        is_l2: true,
        block_time_ms: 1000,
        confirmations: 1,
    },
    // KCC
    ChainInfo {
        chain_id: 321,
        name: "KCC Mainnet",
        symbol: "KCS",
        rpc: "https://rpc-mainnet.kcc.network",
        explorer: "https://explorer.kcc.io",
        is_l2: false,
        block_time_ms: 3000,
        confirmations: 12,
    },
    // zkSync Era
    ChainInfo {
        chain_id: 324,
        name: "zkSync Era",
        symbol: "ETH",
        rpc: "https://mainnet.era.zksync.io",
        explorer: "https://explorer.zksync.io",
        is_l2: true,
        block_time_ms: 1000,
        confirmations: 1,
    },
    // Shiden
    ChainInfo {
        chain_id: 336,
        name: "Shiden",
        symbol: "SDN",
        rpc: "https://shiden.api.onfinality.io/public",
        explorer: "https://shiden.subscan.io",
        is_l2: false,
        block_time_ms: 12000,
        confirmations: 12,
    },
    // Polis
    ChainInfo {
        chain_id: 333999,
        name: "Polis Mainnet",
        symbol: "POLIS",
        rpc: "https://rpc.polis.tech",
        explorer: "https://explorer.polis.tech",
        is_l2: false,
        block_time_ms: 2000,
        confirmations: 12,
    },
    // Theta
    ChainInfo {
        chain_id: 361,
        name: "Theta Mainnet",
        symbol: "TFUEL",
        rpc: "https://eth-rpc-api.thetatoken.org/rpc",
        explorer: "https://explorer.thetatoken.org",
        is_l2: false,
        block_time_ms: 6000,
        confirmations: 12,
    },
    // Wanchain
    ChainInfo {
        chain_id: 888,
        name: "Wanchain",
        symbol: "WAN",
        rpc: "https://gwan-ssl.wandevs.org:56891",
        explorer: "https://www.wanscan.org",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // Callisto
    ChainInfo {
        chain_id: 820,
        name: "Callisto",
        symbol: "CLO",
        rpc: "https://rpc.callisto.network",
        explorer: "https://explorer.callisto.network",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Metis Andromeda
    ChainInfo {
        chain_id: 1088,
        name: "Metis Andromeda",
        symbol: "METIS",
        rpc: "https://andromeda.metis.io/?owner=1088",
        explorer: "https://andromeda-explorer.metis.io",
        is_l2: true,
        block_time_ms: 4000,
        confirmations: 1,
    },
    // Polygon zkEVM
    ChainInfo {
        chain_id: 1101,
        name: "Polygon zkEVM",
        symbol: "ETH",
        rpc: "https://zkevm-rpc.com",
        explorer: "https://zkevm.polygonscan.com",
        is_l2: true,
        block_time_ms: 2000,
        confirmations: 1,
    },
    // Core Chain
    ChainInfo {
        chain_id: 1116,
        name: "Core Chain",
        symbol: "CORE",
        rpc: "https://rpc.coredao.org",
        explorer: "https://scan.coredao.org",
        is_l2: false,
        block_time_ms: 3000,
        confirmations: 12,
    },
    // Moonbeam
    ChainInfo {
        chain_id: 1284,
        name: "Moonbeam",
        symbol: "GLMR",
        rpc: "https://rpc.api.moonbeam.network",
        explorer: "https://moonbeam.moonscan.io",
        is_l2: false,
        block_time_ms: 12000,
        confirmations: 12,
    },
    // Moonriver
    ChainInfo {
        chain_id: 1285,
        name: "Moonriver",
        symbol: "MOVR",
        rpc: "https://rpc.api.moonriver.moonbeam.network",
        explorer: "https://moonriver.moonscan.io",
        is_l2: false,
        block_time_ms: 12000,
        confirmations: 12,
    },
    // Cube
    ChainInfo {
        chain_id: 1818,
        name: "Cube Chain",
        symbol: "CUBE",
        rpc: "https://http-mainnet.cube.network",
        explorer: "https://cubescan.network",
        is_l2: false,
        block_time_ms: 3000,
        confirmations: 12,
    },
    // Aurora
    ChainInfo {
        chain_id: 1313161554,
        name: "Aurora",
        symbol: "ETH",
        rpc: "https://mainnet.aurora.dev",
        explorer: "https://aurorascan.dev",
        is_l2: true,
        block_time_ms: 1000,
        confirmations: 1,
    },
    // Harmony
    ChainInfo {
        chain_id: 1666600000,
        name: "Harmony Shard 0",
        symbol: "ONE",
        rpc: "https://api.harmony.one",
        explorer: "https://explorer.harmony.one",
        is_l2: false,
        block_time_ms: 2000,
        confirmations: 12,
    },
    // IoTeX
    ChainInfo {
        chain_id: 4689,
        name: "IoTeX",
        symbol: "IOTX",
        rpc: "https://babel-api.mainnet.iotex.io",
        explorer: "https://iotexscan.io",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // Smart Bitcoin Cash
    ChainInfo {
        chain_id: 10000,
        name: "Smart Bitcoin Cash",
        symbol: "BCH",
        rpc: "https://smartbch.greyh.at",
        explorer: "https://www.smartscan.cash",
        is_l2: false,
        block_time_ms: 6000,
        confirmations: 12,
    },
    // Palm
    ChainInfo {
        chain_id: 11297108109,
        name: "Palm",
        symbol: "PALM",
        rpc: "https://palm-mainnet.infura.io/v3/",
        explorer: "https://explorer.palm.io",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // Fusion
    ChainInfo {
        chain_id: 32659,
        name: "Fusion Mainnet",
        symbol: "FSN",
        rpc: "https://mainnet.fusionnetwork.io",
        explorer: "https://fsnscan.com",
        is_l2: false,
        block_time_ms: 15000,
        confirmations: 12,
    },
    // Arbitrum Nova
    ChainInfo {
        chain_id: 42170,
        name: "Arbitrum Nova",
        symbol: "ETH",
        rpc: "https://nova.arbitrum.io/rpc",
        explorer: "https://nova.arbiscan.io",
        is_l2: true,
        block_time_ms: 250,
        confirmations: 1,
    },
    // Celo
    ChainInfo {
        chain_id: 42220,
        name: "Celo",
        symbol: "CELO",
        rpc: "https://forno.celo.org",
        explorer: "https://celoscan.io",
        is_l2: false,
        block_time_ms: 5000,
        confirmations: 12,
    },
    // Emerald (Oasis)
    ChainInfo {
        chain_id: 42262,
        name: "Emerald Paratime",
        symbol: "ROSE",
        rpc: "https://emerald.oasis.dev",
        explorer: "https://explorer.emerald.oasis.dev",
        is_l2: false,
        block_time_ms: 6000,
        confirmations: 12,
    },
    // ZKFair
    ChainInfo {
        chain_id: 42766,
        name: "ZKFair",
        symbol: "ZKF",
        rpc: "https://rpc.zkfair.io",
        explorer: "https://scan.zkfair.io",
        is_l2: true,
        block_time_ms: 1000,
        confirmations: 1,
    },
    // Taiko
    ChainInfo {
        chain_id: 167008,
        name: "Taiko Katla",
        symbol: "ETH",
        rpc: "https://rpc.katla.taiko.xyz",
        explorer: "https://explorer.katla.taiko.xyz",
        is_l2: true,
        block_time_ms: 3000,
        confirmations: 1,
    },
    // CMP
    ChainInfo {
        chain_id: 256256,
        name: "CMP Mainnet",
        symbol: "CMP",
        rpc: "https://mainnet.cmpchain.com",
        explorer: "https://explorer.cmpchain.com",
        is_l2: false,
        block_time_ms: 3000,
        confirmations: 12,
    },
    // Scroll
    ChainInfo {
        chain_id: 534352,
        name: "Scroll",
        symbol: "ETH",
        rpc: "https://rpc.scroll.io",
        explorer: "https://scrollscan.com",
        is_l2: true,
        block_time_ms: 3000,
        confirmations: 1,
    },
    // Atlas Sphere (our chain!)
    ChainInfo {
        chain_id: 42,
        name: "Atlas Sphere",
        symbol: "ATLAS",
        rpc: "http://127.0.0.1:9944",
        explorer: "http://explorer.x3",
        is_l2: false,
        block_time_ms: 6000,
        confirmations: 1,
    },
];

/// Get chain info by ID
pub fn get_chain(chain_id: u64) -> Option<&'static ChainInfo> {
    ALL_CHAINS.iter().find(|c| c.chain_id == chain_id)
}

/// Get all chain IDs
pub fn all_chain_ids() -> Vec<u64> {
    ALL_CHAINS.iter().map(|c| c.chain_id).collect()
}

/// Get chains by category
pub fn get_tier1_chains() -> Vec<&'static ChainInfo> {
    ALL_CHAINS
        .iter()
        .filter(|c| {
            matches!(
                c.chain_id,
                1 | 10 | 25 | 56 | 137 | 250 | 8217 | 8453 | 42161 | 43114
            )
        })
        .collect()
}

pub fn get_l2_chains() -> Vec<&'static ChainInfo> {
    ALL_CHAINS.iter().filter(|c| c.is_l2).collect()
}

/// Total chain count
pub fn chain_count() -> usize {
    ALL_CHAINS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_count() {
        assert!(chain_count() > 100, "Should have 100+ chains");
        println!("Total chains registered: {}", chain_count());
    }

    #[test]
    fn test_get_chain() {
        let eth = get_chain(1).unwrap();
        assert_eq!(eth.name, "Ethereum");
        assert_eq!(eth.symbol, "ETH");

        let atlas = get_chain(42).unwrap();
        assert_eq!(atlas.name, "Atlas Sphere");
    }

    #[test]
    fn test_tier1() {
        let tier1 = get_tier1_chains();
        assert_eq!(tier1.len(), 10);
    }

    #[test]
    fn test_l2_chains() {
        let l2s = get_l2_chains();
        assert!(l2s.len() > 10, "Should have many L2s");
        for chain in l2s {
            assert!(chain.is_l2);
        }
    }
}
