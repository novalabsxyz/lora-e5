
/// These tests all require a physical LoRa E5.
/// Run them one at a time to avoid port collisions:
///   ie: cargo test --  --nocapture --test-threads 1

    use super::*;
    use std::str::FromStr;

    fn lora_test_hardware() -> LoraE5<256_usize> {
        const SILICON_LABS_VID: u16 = 0x10C4;
        const CP210X_UART_BRIDGE_PID: u16 = 0xEA60;
        LoraE5::<256>::open_usb(SILICON_LABS_VID, CP210X_UART_BRIDGE_PID).unwrap()
    }

    #[test]
    fn usb_open() {
        let _lora_e5 = lora_test_hardware();
    }

    #[test]
    fn is_ok() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.is_ok().unwrap();
    }

    #[test]
    fn get_version() {
        let mut lora_e5 = lora_test_hardware();
        let _version = lora_e5.get_version().unwrap();
    }

    #[test]
    fn set_subband() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.subband2_only().unwrap();
    }

    #[test]
    fn get_dev_eui() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.get_dev_eui().unwrap();
    }

    #[test]
    fn get_app_eui() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.get_app_eui().unwrap();
    }

    #[test]
    fn set_app_eui() {
        let app_eui = AppEui::from_str("0123456789ABCDEF").unwrap();
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_app_eui(&app_eui).unwrap();
        let fetched_app_eui = lora_e5.get_app_eui().unwrap();
        assert_eq!(app_eui, fetched_app_eui);
    }

    #[test]
    fn set_dev_eui() {
        let dev_eui = DevEui::from_str("111111111111111A").unwrap();
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_dev_eui(&dev_eui).unwrap();
        let fetched_dev_eui = lora_e5.get_dev_eui().unwrap();
        assert_eq!(dev_eui, fetched_dev_eui);
    }

    #[test]
    fn set_app_key() {
        let app_key = AppKey::from_str("111111111111111A111111111111111A").unwrap();
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_app_key(&app_key).unwrap();
    }

    #[test]
    fn set_mode_otaa() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_mode(Mode::Otaa).unwrap();
    }

    #[test]
    fn set_mode_abp() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_mode(Mode::Abp).unwrap();
    }

    #[test]
    fn set_mode_test() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_mode(Mode::Test).unwrap();
    }

    #[test]
    fn set_port() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_port(5).unwrap();
    }

    #[test]
    fn join() {
        let credentials = Credentials::new(
            DevEui::from_str("6081F9A775278564").unwrap(),
            AppEui::from_str("6081F9A498856DCC").unwrap(),
            AppKey::from_str("72F36B996179E634537FCA76047D0B51").unwrap(),
        );
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_mode(Mode::Otaa).unwrap();
        lora_e5.set_region(Region::Us915).unwrap();
        lora_e5.set_credentials(&credentials).unwrap();
        lora_e5.subband2_only().unwrap();
        lora_e5.join().unwrap();
        std::thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn join_and_send() {
        let credentials = Credentials::new(
            DevEui::from_str("6081F9A775278564").unwrap(),
            AppEui::from_str("6081F9A498856DCC").unwrap(),
            AppKey::from_str("72F36B996179E634537FCA76047D0B51").unwrap(),
        );
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_mode(Mode::Otaa).unwrap();
        lora_e5.set_region(Region::Us915).unwrap();
        lora_e5.set_credentials(&credentials).unwrap();
        lora_e5.subband2_only().unwrap();
        assert!(lora_e5.join().unwrap());
        lora_e5.send(&[1, 2, 3, 4], 3, true).unwrap();
    }

#[test]
fn parse_signal() {
    let response="+CMSGHEX: Start\r
    +CMSGHEX: Wait ACK\r
    +CMSGHEX: FPENDING\r
    +CMSGHEX: ACK Received\r
    +CMSGHEX: RXWIN1, RSSI -79, SNR 7.0\r
    +CMSGHEX: Done\r
";
    if let Some(m) = response.find("RXWIN1") {
        let (rssi, snr) = parse_rssi_snr(&response,m ).unwrap();
        assert_eq!(rssi, -79);
        assert_eq!(snr, 7.0);
    } else {
        assert!(false)
    }
}
