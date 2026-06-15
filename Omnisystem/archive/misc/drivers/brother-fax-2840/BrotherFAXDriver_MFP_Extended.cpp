#include "BrotherFAXDriver_MFP_Extended.hpp"
#include <string.h>

// MARK: - OSObject Methods

OSDefineMetaClassAndStructors(BrotherFAXDriverMFP, BrotherFAXDriver);

// MARK: - Initialization

kern_return_t BrotherFAXDriverMFP::InitializePrinterMaps() {
    // Resolution mapping (DPI → device code)
    resolution_map_[300]   = PRINTER_RESOLUTION_300DPI;
    resolution_map_[600]   = PRINTER_RESOLUTION_600DPI;
    resolution_map_[1200]  = PRINTER_RESOLUTION_1200DPI;

    // Paper size mapping (from Linux CUPS wrapper)
    pagesize_map_["A4"]         = PRINTER_PAGESIZE_A4;
    pagesize_map_["Letter"]     = PRINTER_PAGESIZE_LETTER;
    pagesize_map_["Legal"]      = PRINTER_PAGESIZE_LEGAL;
    pagesize_map_["A5"]         = PRINTER_PAGESIZE_A5;
    pagesize_map_["A6"]         = PRINTER_PAGESIZE_A6;
    pagesize_map_["Postcard"]   = PRINTER_PAGESIZE_POSTCARD;
    pagesize_map_["B5"]         = PRINTER_PAGESIZE_B5;
    pagesize_map_["Envelope"]   = PRINTER_PAGESIZE_ENVELOPE;

    // Media type mapping
    mediatype_map_["Plain"]       = PRINTER_MEDIA_PLAIN;
    mediatype_map_["Thin"]        = PRINTER_MEDIA_THIN;
    mediatype_map_["Thick"]       = PRINTER_MEDIA_THICK;
    mediatype_map_["Bond"]        = PRINTER_MEDIA_BOND;
    mediatype_map_["Transparency"]= PRINTER_MEDIA_TRANS;
    mediatype_map_["Envelope"]    = PRINTER_MEDIA_ENVELOPE;

    // Paper source mapping
    papersource_map_["Auto"]      = PRINTER_SOURCE_AUTO;
    papersource_map_["Manual"]    = PRINTER_SOURCE_MANUAL;
    papersource_map_["Tray1"]     = PRINTER_SOURCE_TRAY1;
    papersource_map_["Tray2"]     = PRINTER_SOURCE_TRAY2;
    papersource_map_["Tray3"]     = PRINTER_SOURCE_TRAY3;
    papersource_map_["MPTray"]    = PRINTER_SOURCE_MPTRAY;

    return KERN_SUCCESS;
}

// MARK: - Printer Operations (New)

kern_return_t BrotherFAXDriverMFP::SendPrinterData(const uint8_t* data, uint32_t length, uint32_t page_number) {
    if (!data || length == 0) {
        os_log_error(logger_, "BrotherFAXDriverMFP::SendPrinterData - Invalid parameters");
        return kIOReturnBadArgument;
    }

    if (device_state_ != DeviceState::Idle && printer_state_ != PrinterMode::Printing) {
        os_log_error(logger_, "BrotherFAXDriverMFP::SendPrinterData - Device not ready for printing");
        return kIOReturnNotReady;
    }

    // Transition to printing
    HandlePrinterStateTransition(PrinterMode::Printing);
    current_page_number_ = page_number;

    // Send printer data via bulk endpoint (same as fax)
    IOUSBHostPipe* pipe = GetEndpoint(EP_BULK_OUT);
    if (!pipe) {
        os_log_error(logger_, "BrotherFAXDriverMFP::SendPrinterData - Failed to get bulk OUT endpoint");
        HandlePrinterStateTransition(PrinterMode::ErrorPrint);
        return kIOReturnNoDevice;
    }

    kern_return_t status = pipe->Write(const_cast<uint8_t*>(data), length, device_timeout_ms_, nullptr);
    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriverMFP::SendPrinterData - Bulk write failed: 0x%x", status);
        HandlePrinterStateTransition(PrinterMode::ErrorPrint);
        return status;
    }

    os_log(logger_, "BrotherFAXDriverMFP::SendPrinterData - Sent %u bytes (page %u)", length, page_number);

    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriverMFP::SetPrinterConfiguration(const PrinterConfiguration& config) {
    os_log(logger_, "BrotherFAXDriverMFP::SetPrinterConfiguration - Configuring printer");

    if (device_state_ != DeviceState::Idle) {
        os_log_error(logger_, "BrotherFAXDriverMFP::SetPrinterConfiguration - Device not idle");
        return kIOReturnNotReady;
    }

    // Encode configuration into device format
    uint8_t config_bytes[32];
    uint32_t config_length = 0;
    kern_return_t status = EncodePrinterConfiguration(config, config_bytes, &config_length);
    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriverMFP::SetPrinterConfiguration - Encoding failed");
        return status;
    }

    // Send configuration via control transfer (SET_PRINTER_CONFIG)
    status = PerformPrinterControlTransfer(
        0x21,  // bmRequestType (class, interface, OUT)
        0x03,  // bRequest (SET_PRINTER_CONFIG) – inferred from Windows driver
        0x0000, // wValue
        0x0000, // wIndex
        config_bytes,
        config_length
    );

    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriverMFP::SetPrinterConfiguration - Control transfer failed: 0x%x", status);
        return status;
    }

    // Store current configuration
    current_printer_config_ = config;

    os_log(logger_, "BrotherFAXDriverMFP::SetPrinterConfiguration - Configuration applied successfully");
    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriverMFP::GetPrinterStatus(PrinterStatus* status) {
    if (!status) {
        os_log_error(logger_, "BrotherFAXDriverMFP::GetPrinterStatus - Invalid parameter");
        return kIOReturnBadArgument;
    }

    os_log(logger_, "BrotherFAXDriverMFP::GetPrinterStatus - Querying printer status");

    // Query printer status via control transfer (GET_PRINTER_STATUS)
    uint8_t status_bytes[8];
    kern_return_t result = PerformPrinterControlTransfer(
        0xA1,  // bmRequestType (class, interface, IN)
        0x04,  // bRequest (GET_PRINTER_STATUS) – inferred
        0x0000, // wValue
        0x0000, // wIndex
        status_bytes,
        sizeof(status_bytes)
    );

    if (result != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriverMFP::GetPrinterStatus - Control transfer failed: 0x%x", result);
        return result;
    }

    // Decode status bytes
    result = DecodePrinterStatus(status_bytes, sizeof(status_bytes), status);
    if (result != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriverMFP::GetPrinterStatus - Decoding failed");
        return result;
    }

    os_log(logger_, "BrotherFAXDriverMFP::GetPrinterStatus - Toner: %u%%, Jam: %d, Temp: %u°C",
           status->toner_level, status->paper_jam, status->temperature);

    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriverMFP::EjectPage() {
    os_log(logger_, "BrotherFAXDriverMFP::EjectPage - Ejecting page");

    if (printer_state_ != PrinterMode::Printing && printer_state_ != PrinterMode::Idle) {
        os_log_error(logger_, "BrotherFAXDriverMFP::EjectPage - Printer not ready to eject");
        return kIOReturnNotReady;
    }

    // Send eject command via control transfer
    kern_return_t status = PerformPrinterControlTransfer(
        0x21,  // bmRequestType (class, interface, OUT)
        0x05,  // bRequest (EJECT_PAGE) – inferred
        0x0000, // wValue
        0x0000, // wIndex
        nullptr,
        0
    );

    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriverMFP::EjectPage - Control transfer failed: 0x%x", status);
        return status;
    }

    // Transition back to idle
    HandlePrinterStateTransition(PrinterMode::Idle);

    os_log(logger_, "BrotherFAXDriverMFP::EjectPage - Page ejected successfully");
    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriverMFP::CancelPrintJob() {
    os_log(logger_, "BrotherFAXDriverMFP::CancelPrintJob - Canceling print job");

    if (printer_state_ == PrinterMode::Idle) {
        os_log(logger_, "BrotherFAXDriverMFP::CancelPrintJob - No active print job");
        return KERN_SUCCESS;
    }

    // Reset endpoint to cancel any pending transfers
    ResetEndpoint(EP_BULK_OUT);

    // Transition to idle
    HandlePrinterStateTransition(PrinterMode::Idle);
    current_print_job_id_ = 0;

    os_log(logger_, "BrotherFAXDriverMFP::CancelPrintJob - Print job canceled");
    return KERN_SUCCESS;
}

// MARK: - Configuration Management

kern_return_t BrotherFAXDriverMFP::ConfigurePrinter(uint32_t resolution_dpi, const char* paper_size,
                                                     bool duplex, const char* media_type) {
    if (!paper_size || !media_type) {
        return kIOReturnBadArgument;
    }

    PrinterConfiguration config;

    // Set resolution
    auto res_it = resolution_map_.find(resolution_dpi);
    if (res_it == resolution_map_.end()) {
        os_log_error(logger_, "BrotherFAXDriverMFP::ConfigurePrinter - Invalid resolution: %u", resolution_dpi);
        return kIOReturnBadArgument;
    }
    config.resolution = res_it->second;

    // Set paper size
    auto page_it = pagesize_map_.find(paper_size);
    if (page_it == pagesize_map_.end()) {
        os_log_error(logger_, "BrotherFAXDriverMFP::ConfigurePrinter - Invalid paper size: %s", paper_size);
        return kIOReturnBadArgument;
    }
    config.page_size = page_it->second;

    // Set media type
    auto media_it = mediatype_map_.find(media_type);
    if (media_it == mediatype_map_.end()) {
        os_log_error(logger_, "BrotherFAXDriverMFP::ConfigurePrinter - Invalid media type: %s", media_type);
        return kIOReturnBadArgument;
    }
    config.media_type = media_it->second;

    // Set duplex
    config.duplex_mode = duplex ? PRINTER_DUPLEX_LONGEDGE : PRINTER_DUPLEX_NONE;

    // Set other defaults
    config.paper_source = PRINTER_SOURCE_AUTO;
    config.toner_save_mode = false;
    config.brightness = 100;

    // Apply configuration
    return SetPrinterConfiguration(config);
}

// MARK: - Helper Methods

kern_return_t BrotherFAXDriverMFP::EncodePrinterConfiguration(const PrinterConfiguration& config,
                                                              uint8_t* config_bytes, uint32_t* config_length) {
    if (!config_bytes || !config_length) {
        return kIOReturnBadArgument;
    }

    // Brother proprietary format (inferred from CUPS wrapper mappings)
    // Byte 0: Resolution
    // Byte 1: Page size
    // Byte 2: Media type
    // Byte 3: Paper source
    // Byte 4: Duplex mode | (toner_save << 4)
    // Byte 5: Brightness

    config_bytes[0] = config.resolution;
    config_bytes[1] = config.page_size;
    config_bytes[2] = config.media_type;
    config_bytes[3] = config.paper_source;
    config_bytes[4] = config.duplex_mode | (config.toner_save_mode ? 0x10 : 0x00);
    config_bytes[5] = config.brightness;

    *config_length = 6;

    os_log(logger_, "BrotherFAXDriverMFP::EncodePrinterConfiguration - Encoded %u bytes", *config_length);

    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriverMFP::DecodePrinterStatus(const uint8_t* status_bytes, uint32_t length,
                                                       PrinterStatus* status) {
    if (!status_bytes || length < 5 || !status) {
        return kIOReturnBadArgument;
    }

    // Brother proprietary format (inferred)
    // Byte 0: Toner level (0-100%)
    // Byte 1: Error flags (bit 0=jam, bit 1=toner_low, bit 2=door_open)
    // Byte 2: Temperature (Celsius)
    // Bytes 3-6: Page count (little-endian)

    status->toner_level = status_bytes[0];
    status->paper_jam = (status_bytes[1] & 0x01) ? true : false;
    status->toner_low = (status_bytes[1] & 0x02) ? true : false;
    status->door_open = (status_bytes[1] & 0x04) ? true : false;
    status->temperature = status_bytes[2];
    status->page_count = (status_bytes[3]) |
                         (status_bytes[4] << 8) |
                         (status_bytes[5] << 16) |
                         (status_bytes[6] << 24);
    status->error_code = status_bytes[7];

    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriverMFP::PerformPrinterControlTransfer(uint8_t request_type, uint8_t request,
                                                                 uint16_t value, uint16_t index,
                                                                 void* data, uint16_t length) {
    IOUSBDeviceRequest device_request;
    device_request.bmRequestType = request_type;
    device_request.bRequest = request;
    device_request.wValue = OSSwapHostToLittleInt16(value);
    device_request.wIndex = OSSwapHostToLittleInt16(index);
    device_request.wLength = OSSwapHostToLittleInt16(length);

    if (data && length > 0) {
        device_request.pData = data;
    }

    kern_return_t status = DeviceRequest(&device_request);
    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriverMFP::PerformPrinterControlTransfer - Failed: 0x%x", status);
        return status;
    }

    return KERN_SUCCESS;
}

// MARK: - State Machine Extensions

void BrotherFAXDriverMFP::HandlePrinterStateTransition(PrinterMode new_state) {
    PrinterMode old_state = printer_state_;
    printer_state_ = new_state;

    os_log(logger_, "BrotherFAXDriverMFP::HandlePrinterStateTransition - %d -> %d",
           static_cast<int>(old_state), static_cast<int>(new_state));

    switch (printer_state_) {
        case PrinterMode::Idle:
            os_log(logger_, "BrotherFAXDriverMFP: Printer idle");
            break;

        case PrinterMode::Printing:
            os_log(logger_, "BrotherFAXDriverMFP: Printer printing (page %u)", current_page_number_);
            break;

        case PrinterMode::ConfiguringPrint:
            os_log(logger_, "BrotherFAXDriverMFP: Configuring printer");
            break;

        case PrinterMode::ErrorPrint:
            os_log_error(logger_, "BrotherFAXDriverMFP: Printer error state");
            break;

        default:
            os_log_error(logger_, "BrotherFAXDriverMFP: Unknown printer state: %d", static_cast<int>(new_state));
            break;
    }
}

#ifdef __cplusplus
}
#endif
