#ifndef BROTHERFAXDRIVER_HPP
#define BROTHERFAXDRIVER_HPP

#include <DriverKit/DriverKit.h>
#include <DriverKit/IOUSBHostDevice.h>
#include <DriverKit/IOUSBHostPipe.h>
#include <DriverKit/OSAction.h>
#include <os/log.h>
#include <string.h>

// MARK: - Device Constants
#define BROTHER_VENDOR_ID    0x04F9
#define BROTHER_DEVICE_ID    0x0346
#define BROTHER_FAX_2840_MAX_PACKET_SIZE 512
#define BROTHER_FAX_TIMEOUT_MS           30000
#define BROTHER_STATUS_POLL_INTERVAL_MS  10

// MARK: - Endpoint Addresses
#define EP_BULK_OUT         0x01
#define EP_BULK_IN          0x82
#define EP_INTERRUPT_IN     0x83

// MARK: - Status Codes
#define FAX_STATUS_IDLE         0x00
#define FAX_STATUS_TRANSMITTING 0x01
#define FAX_STATUS_RECEIVING    0x02
#define FAX_STATUS_ERROR        0xFF

class BrotherFAXDriver : public IOUSBHostDevice {
    OSDeclareDefaultStructors(BrotherFAXDriver);

public:
    // DriverKit required methods
    virtual kern_return_t Start(IOService* provider) final;
    virtual kern_return_t Stop(IOService* provider) final;
    virtual kern_return_t SetProperties(OSDictionary* properties) final;

    // Fax operations (from DIS)
    kern_return_t InitDevice();
    kern_return_t SendFaxData(const uint8_t* data, uint32_t length);
    kern_return_t ReceiveFaxData(uint8_t* buffer, uint32_t buffer_size, uint32_t* bytes_read);
    kern_return_t GetDeviceStatus(uint8_t* status_byte);
    kern_return_t ResetDevice();
    kern_return_t GetDeviceID(char* device_id_str, uint32_t max_length);

protected:
    // Endpoint management
    IOUSBHostPipe* GetEndpoint(uint8_t address);
    void ReleaseEndpoints();

    // State machine handling
    void HandleStateTransition(uint8_t new_state);

    // Interrupt handling
    kern_return_t ScheduleStatusPolling();
    kern_return_t CancelStatusPolling();

private:
    // Device state enumeration
    enum class DeviceState : uint8_t {
        Uninitialized = 0,
        Idle = 1,
        Transmitting = 2,
        Receiving = 3,
        Error = 4,
    };

    // Member variables
    DeviceState device_state_ = DeviceState::Uninitialized;
    IOUSBHostPipe* bulk_out_pipe_ = nullptr;
    IOUSBHostPipe* bulk_in_pipe_ = nullptr;
    IOUSBHostPipe* interrupt_in_pipe_ = nullptr;
    os_log_t logger_ = nullptr;

    // Configuration
    bool interrupt_polling_active_ = false;
    uint32_t device_timeout_ms_ = BROTHER_FAX_TIMEOUT_MS;

    // Helper methods
    kern_return_t ConfigureEndpoints();
    kern_return_t ResetEndpoint(uint8_t endpoint_address);
    kern_return_t PerformControlTransfer(
        uint8_t request_type,
        uint8_t request,
        uint16_t value,
        uint16_t index,
        void* data,
        uint16_t length
    );
};

#endif // BROTHERFAXDRIVER_HPP
