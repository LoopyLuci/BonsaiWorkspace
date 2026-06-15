#include "BrotherFAXDriver.hpp"

// MARK: - OSObject Methods

OSDefineMetaClassAndStructors(BrotherFAXDriver, IOUSBHostDevice);

// MARK: - DriverKit Lifecycle

kern_return_t BrotherFAXDriver::Start(IOService* provider) {
    kern_return_t status = super::Start(provider);
    if (status != KERN_SUCCESS) {
        os_log_error(OS_LOG_DEFAULT, "BrotherFAXDriver: super::Start failed: 0x%x", status);
        return status;
    }

    logger_ = os_log_create("com.omnisystem.brotherfaxdriver", "driver");
    os_log(logger_, "BrotherFAXDriver::Start - Initializing device");

    // Configure endpoints (bulk IN/OUT, interrupt IN)
    status = ConfigureEndpoints();
    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriver: ConfigureEndpoints failed: 0x%x", status);
        Stop(provider);
        return status;
    }

    // Initialize device
    status = InitDevice();
    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriver: InitDevice failed: 0x%x", status);
        Stop(provider);
        return status;
    }

    // Start status polling
    status = ScheduleStatusPolling();
    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriver: ScheduleStatusPolling failed: 0x%x", status);
    }

    os_log(logger_, "BrotherFAXDriver::Start - Device initialized successfully");
    RegisterService();

    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriver::Stop(IOService* provider) {
    os_log(logger_, "BrotherFAXDriver::Stop - Shutting down");

    CancelStatusPolling();
    ReleaseEndpoints();

    kern_return_t status = super::Stop(provider);

    if (logger_) {
        os_release(logger_);
        logger_ = nullptr;
    }

    return status;
}

kern_return_t BrotherFAXDriver::SetProperties(OSDictionary* properties) {
    os_log(logger_, "BrotherFAXDriver::SetProperties called");
    return super::SetProperties(properties);
}

// MARK: - Fax Operations (from DIS)

kern_return_t BrotherFAXDriver::InitDevice() {
    os_log(logger_, "BrotherFAXDriver::InitDevice - Starting initialization");

    // Rule 4: Control Transfer - SET_PORT_STATUS
    // bRequest=1 (SET_PORT_STATUS), bmRequestType=0x21 (class, interface, OUT)
    kern_return_t status = PerformControlTransfer(
        0x21,  // bmRequestType
        0x01,  // bRequest (SET_PORT_STATUS)
        0x0000, // wValue
        0x0000, // wIndex
        nullptr, // no data
        0      // wLength
    );

    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriver::InitDevice - Control transfer failed: 0x%x", status);
        return status;
    }

    // State transition: uninitialized -> idle
    HandleStateTransition(static_cast<uint8_t>(DeviceState::Idle));
    os_log(logger_, "BrotherFAXDriver::InitDevice - Device initialized successfully");

    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriver::SendFaxData(const uint8_t* data, uint32_t length) {
    if (!data || length == 0) {
        os_log_error(logger_, "BrotherFAXDriver::SendFaxData - Invalid parameters");
        return kIOReturnBadArgument;
    }

    if (device_state_ != DeviceState::Idle && device_state_ != DeviceState::Transmitting) {
        os_log_error(logger_, "BrotherFAXDriver::SendFaxData - Device not in transmit state: %d", static_cast<int>(device_state_));
        return kIOReturnNotReady;
    }

    // Transition to transmitting
    HandleStateTransition(static_cast<uint8_t>(DeviceState::Transmitting));

    // Rule 1: Bulk Write to endpoint 0x01
    IOUSBHostPipe* pipe = GetEndpoint(EP_BULK_OUT);
    if (!pipe) {
        os_log_error(logger_, "BrotherFAXDriver::SendFaxData - Failed to get bulk OUT endpoint");
        HandleStateTransition(static_cast<uint8_t>(DeviceState::Error));
        return kIOReturnNoDevice;
    }

    kern_return_t status = pipe->Write(const_cast<uint8_t*>(data), length, device_timeout_ms_, nullptr);
    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriver::SendFaxData - Bulk write failed: 0x%x", status);
        HandleStateTransition(static_cast<uint8_t>(DeviceState::Error));
        return status;
    }

    os_log(logger_, "BrotherFAXDriver::SendFaxData - Sent %u bytes successfully", length);

    // Transition back to idle after successful transfer
    HandleStateTransition(static_cast<uint8_t>(DeviceState::Idle));

    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriver::ReceiveFaxData(uint8_t* buffer, uint32_t buffer_size, uint32_t* bytes_read) {
    if (!buffer || buffer_size == 0 || !bytes_read) {
        os_log_error(logger_, "BrotherFAXDriver::ReceiveFaxData - Invalid parameters");
        return kIOReturnBadArgument;
    }

    if (device_state_ != DeviceState::Idle && device_state_ != DeviceState::Receiving) {
        os_log_error(logger_, "BrotherFAXDriver::ReceiveFaxData - Device not in receive state: %d", static_cast<int>(device_state_));
        return kIOReturnNotReady;
    }

    // Transition to receiving
    HandleStateTransition(static_cast<uint8_t>(DeviceState::Receiving));

    // Rule 2: Bulk Read from endpoint 0x82
    IOUSBHostPipe* pipe = GetEndpoint(EP_BULK_IN);
    if (!pipe) {
        os_log_error(logger_, "BrotherFAXDriver::ReceiveFaxData - Failed to get bulk IN endpoint");
        HandleStateTransition(static_cast<uint8_t>(DeviceState::Error));
        return kIOReturnNoDevice;
    }

    uint32_t bytes_transferred = 0;
    kern_return_t status = pipe->Read(buffer, buffer_size, &bytes_transferred, device_timeout_ms_, nullptr);

    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriver::ReceiveFaxData - Bulk read failed: 0x%x", status);
        HandleStateTransition(static_cast<uint8_t>(DeviceState::Error));
        return status;
    }

    *bytes_read = bytes_transferred;
    os_log(logger_, "BrotherFAXDriver::ReceiveFaxData - Received %u bytes successfully", bytes_transferred);

    // Transition back to idle after successful transfer
    HandleStateTransition(static_cast<uint8_t>(DeviceState::Idle));

    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriver::GetDeviceStatus(uint8_t* status_byte) {
    if (!status_byte) {
        os_log_error(logger_, "BrotherFAXDriver::GetDeviceStatus - Invalid parameter");
        return kIOReturnBadArgument;
    }

    // Rule 3: Interrupt Read from endpoint 0x83
    IOUSBHostPipe* pipe = GetEndpoint(EP_INTERRUPT_IN);
    if (!pipe) {
        os_log_error(logger_, "BrotherFAXDriver::GetDeviceStatus - Failed to get interrupt IN endpoint");
        return kIOReturnNoDevice;
    }

    uint8_t status_buffer[8];
    uint32_t bytes_transferred = 0;
    kern_return_t status = pipe->Read(status_buffer, sizeof(status_buffer), &bytes_transferred, 10000, nullptr);

    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriver::GetDeviceStatus - Interrupt read failed: 0x%x", status);
        return status;
    }

    if (bytes_transferred > 0) {
        *status_byte = status_buffer[0];
        os_log(logger_, "BrotherFAXDriver::GetDeviceStatus - Status: 0x%02x", *status_byte);

        // Handle error state transitions
        if (*status_byte == FAX_STATUS_ERROR) {
            HandleStateTransition(static_cast<uint8_t>(DeviceState::Error));
        }
    }

    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriver::ResetDevice() {
    os_log(logger_, "BrotherFAXDriver::ResetDevice - Resetting device");

    // Reset all endpoints
    kern_return_t status = ResetEndpoint(EP_BULK_OUT);
    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriver::ResetDevice - Failed to reset bulk OUT endpoint: 0x%x", status);
    }

    status = ResetEndpoint(EP_BULK_IN);
    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriver::ResetDevice - Failed to reset bulk IN endpoint: 0x%x", status);
    }

    // Transition back to idle
    HandleStateTransition(static_cast<uint8_t>(DeviceState::Idle));

    os_log(logger_, "BrotherFAXDriver::ResetDevice - Device reset complete");
    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriver::GetDeviceID(char* device_id_str, uint32_t max_length) {
    if (!device_id_str || max_length == 0) {
        os_log_error(logger_, "BrotherFAXDriver::GetDeviceID - Invalid parameters");
        return kIOReturnBadArgument;
    }

    // Rule 4: Control Transfer - GET_DEVICE_ID (IEEE 1284)
    uint8_t device_id_buffer[256];
    kern_return_t status = PerformControlTransfer(
        0xC0,  // bmRequestType (vendor, device, IN)
        0x00,  // bRequest (GET_DEVICE_ID)
        0x0000, // wValue
        0x0000, // wIndex
        device_id_buffer, // data buffer
        sizeof(device_id_buffer) // wLength
    );

    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriver::GetDeviceID - Control transfer failed: 0x%x", status);
        return status;
    }

    // Copy device ID string, respecting max_length
    uint32_t copy_length = (sizeof(device_id_buffer) < max_length) ? sizeof(device_id_buffer) : max_length - 1;
    memcpy(device_id_str, device_id_buffer, copy_length);
    device_id_str[copy_length] = '\0';

    os_log(logger_, "BrotherFAXDriver::GetDeviceID - Device ID: %s", device_id_str);
    return KERN_SUCCESS;
}

// MARK: - Protected Methods

IOUSBHostPipe* BrotherFAXDriver::GetEndpoint(uint8_t address) {
    switch (address) {
        case EP_BULK_OUT:
            return bulk_out_pipe_;
        case EP_BULK_IN:
            return bulk_in_pipe_;
        case EP_INTERRUPT_IN:
            return interrupt_in_pipe_;
        default:
            os_log_error(logger_, "BrotherFAXDriver::GetEndpoint - Unknown endpoint: 0x%02x", address);
            return nullptr;
    }
}

void BrotherFAXDriver::ReleaseEndpoints() {
    if (bulk_out_pipe_) {
        bulk_out_pipe_->Abort();
        bulk_out_pipe_->release();
        bulk_out_pipe_ = nullptr;
    }

    if (bulk_in_pipe_) {
        bulk_in_pipe_->Abort();
        bulk_in_pipe_->release();
        bulk_in_pipe_ = nullptr;
    }

    if (interrupt_in_pipe_) {
        interrupt_in_pipe_->Abort();
        interrupt_in_pipe_->release();
        interrupt_in_pipe_ = nullptr;
    }
}

void BrotherFAXDriver::HandleStateTransition(uint8_t new_state) {
    DeviceState old_state = device_state_;
    device_state_ = static_cast<DeviceState>(new_state);

    os_log(logger_, "BrotherFAXDriver::HandleStateTransition - %d -> %d",
           static_cast<int>(old_state), static_cast<int>(device_state_));

    // State machine transitions from DIS
    switch (device_state_) {
        case DeviceState::Idle:
            os_log(logger_, "BrotherFAXDriver: Device idle, ready for operations");
            break;

        case DeviceState::Transmitting:
            os_log(logger_, "BrotherFAXDriver: Device transmitting");
            break;

        case DeviceState::Receiving:
            os_log(logger_, "BrotherFAXDriver: Device receiving");
            break;

        case DeviceState::Error:
            os_log_error(logger_, "BrotherFAXDriver: Device entered error state");
            break;

        default:
            os_log_error(logger_, "BrotherFAXDriver: Unknown state: %d", static_cast<int>(device_state_));
            break;
    }
}

kern_return_t BrotherFAXDriver::ScheduleStatusPolling() {
    os_log(logger_, "BrotherFAXDriver::ScheduleStatusPolling - Starting status polling");
    interrupt_polling_active_ = true;
    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriver::CancelStatusPolling() {
    os_log(logger_, "BrotherFAXDriver::CancelStatusPolling - Canceling status polling");
    interrupt_polling_active_ = false;
    return KERN_SUCCESS;
}

// MARK: - Private Methods

kern_return_t BrotherFAXDriver::ConfigureEndpoints() {
    os_log(logger_, "BrotherFAXDriver::ConfigureEndpoints - Configuring USB endpoints");

    kern_return_t status = KERN_SUCCESS;

    // Get bulk OUT endpoint (0x01)
    bulk_out_pipe_ = CopyPipeForEndpoint(EP_BULK_OUT);
    if (!bulk_out_pipe_) {
        os_log_error(logger_, "BrotherFAXDriver::ConfigureEndpoints - Failed to get bulk OUT endpoint");
        return kIOReturnNoDevice;
    }
    os_log(logger_, "BrotherFAXDriver::ConfigureEndpoints - Bulk OUT endpoint configured");

    // Get bulk IN endpoint (0x82)
    bulk_in_pipe_ = CopyPipeForEndpoint(EP_BULK_IN);
    if (!bulk_in_pipe_) {
        os_log_error(logger_, "BrotherFAXDriver::ConfigureEndpoints - Failed to get bulk IN endpoint");
        bulk_out_pipe_->release();
        bulk_out_pipe_ = nullptr;
        return kIOReturnNoDevice;
    }
    os_log(logger_, "BrotherFAXDriver::ConfigureEndpoints - Bulk IN endpoint configured");

    // Get interrupt IN endpoint (0x83)
    interrupt_in_pipe_ = CopyPipeForEndpoint(EP_INTERRUPT_IN);
    if (!interrupt_in_pipe_) {
        os_log_error(logger_, "BrotherFAXDriver::ConfigureEndpoints - Failed to get interrupt IN endpoint");
        bulk_out_pipe_->release();
        bulk_in_pipe_->release();
        bulk_out_pipe_ = nullptr;
        bulk_in_pipe_ = nullptr;
        return kIOReturnNoDevice;
    }
    os_log(logger_, "BrotherFAXDriver::ConfigureEndpoints - Interrupt IN endpoint configured");

    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriver::ResetEndpoint(uint8_t endpoint_address) {
    IOUSBHostPipe* pipe = GetEndpoint(endpoint_address);
    if (!pipe) {
        return kIOReturnNoDevice;
    }

    kern_return_t status = pipe->Abort();
    if (status != KERN_SUCCESS) {
        os_log_error(logger_, "BrotherFAXDriver::ResetEndpoint - Abort failed: 0x%x", status);
        return status;
    }

    os_log(logger_, "BrotherFAXDriver::ResetEndpoint - Endpoint 0x%02x reset", endpoint_address);
    return KERN_SUCCESS;
}

kern_return_t BrotherFAXDriver::PerformControlTransfer(
    uint8_t request_type,
    uint8_t request,
    uint16_t value,
    uint16_t index,
    void* data,
    uint16_t length
) {
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
        os_log_error(logger_, "BrotherFAXDriver::PerformControlTransfer - Failed: 0x%x", status);
        return status;
    }

    os_log(logger_, "BrotherFAXDriver::PerformControlTransfer - Success");
    return KERN_SUCCESS;
}
