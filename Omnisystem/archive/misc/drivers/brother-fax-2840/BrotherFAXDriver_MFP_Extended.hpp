#ifndef BROTHERFAXDRIVER_MFP_EXTENDED_HPP
#define BROTHERFAXDRIVER_MFP_EXTENDED_HPP

#include "BrotherFAXDriver.hpp"
#include <map>
#include <queue>

// MARK: - Printer Configuration Types

#define PRINTER_RESOLUTION_300DPI    0x01
#define PRINTER_RESOLUTION_600DPI    0x02
#define PRINTER_RESOLUTION_1200DPI   0x04
#define PRINTER_RESOLUTION_HQ1200    0x08

#define PRINTER_PAGESIZE_A4          0x00
#define PRINTER_PAGESIZE_LETTER      0x01
#define PRINTER_PAGESIZE_LEGAL       0x02
#define PRINTER_PAGESIZE_A5          0x03
#define PRINTER_PAGESIZE_A6          0x04
#define PRINTER_PAGESIZE_POSTCARD    0x05
#define PRINTER_PAGESIZE_B5          0x06
#define PRINTER_PAGESIZE_ENVELOPE    0x07

#define PRINTER_MEDIA_PLAIN          0x00
#define PRINTER_MEDIA_THIN           0x01
#define PRINTER_MEDIA_THICK          0x02
#define PRINTER_MEDIA_BOND           0x03
#define PRINTER_MEDIA_TRANS          0x04
#define PRINTER_MEDIA_ENVELOPE       0x05

#define PRINTER_SOURCE_AUTO          0x00
#define PRINTER_SOURCE_MANUAL        0x01
#define PRINTER_SOURCE_TRAY1         0x02
#define PRINTER_SOURCE_TRAY2         0x03
#define PRINTER_SOURCE_TRAY3         0x04
#define PRINTER_SOURCE_MPTRAY        0x05

#define PRINTER_DUPLEX_NONE          0x00
#define PRINTER_DUPLEX_LONGEDGE      0x01
#define PRINTER_DUPLEX_SHORTEDGE     0x02

// MARK: - Printer Configuration Struct

struct PrinterConfiguration {
    uint8_t resolution;      // PRINTER_RESOLUTION_*
    uint8_t page_size;       // PRINTER_PAGESIZE_*
    uint8_t media_type;      // PRINTER_MEDIA_*
    uint8_t paper_source;    // PRINTER_SOURCE_*
    uint8_t duplex_mode;     // PRINTER_DUPLEX_*
    bool toner_save_mode;
    uint8_t brightness;      // 0-100
};

struct PrinterStatus {
    uint8_t toner_level;     // 0-100%
    bool paper_jam;
    bool toner_low;
    bool door_open;
    uint8_t temperature;     // Celsius
    uint32_t page_count;     // Lifetime pages
    uint8_t error_code;      // 0 = no error
};

// MARK: - Extended MFP Driver Class

class BrotherFAXDriverMFP : public BrotherFAXDriver {
    OSDeclareDefaultStructors(BrotherFAXDriverMFP);

public:
    // DriverKit lifecycle (inherited from BrotherFAXDriver)

    // Printer operations (new)
    kern_return_t SendPrinterData(const uint8_t* data, uint32_t length, uint32_t page_number);
    kern_return_t SetPrinterConfiguration(const PrinterConfiguration& config);
    kern_return_t GetPrinterStatus(PrinterStatus* status);
    kern_return_t EjectPage();
    kern_return_t CancelPrintJob();

    // Configuration management
    kern_return_t ConfigurePrinter(uint32_t resolution_dpi, const char* paper_size,
                                   bool duplex, const char* media_type);

protected:
    // Printer state management
    enum class PrinterMode {
        Idle = 0,
        Printing = 1,
        ConfiguringPrint = 2,
        ErrorPrint = 3,
    };

    void HandlePrinterStateTransition(PrinterMode new_state);

    // Helper methods
    kern_return_t EncodePrinterConfiguration(const PrinterConfiguration& config,
                                             uint8_t* config_bytes, uint32_t* config_length);
    kern_return_t DecodePrinterStatus(const uint8_t* status_bytes, uint32_t length,
                                      PrinterStatus* status);
    kern_return_t PerformPrinterControlTransfer(uint8_t request_type, uint8_t request,
                                                uint16_t value, uint16_t index,
                                                void* data, uint16_t length);

private:
    // Printer state variables
    PrinterMode printer_state_ = PrinterMode::Idle;
    PrinterConfiguration current_printer_config_;
    uint32_t current_print_job_id_ = 0;
    uint32_t pages_in_current_job_ = 0;
    uint32_t current_page_number_ = 0;

    // Printer data queue (for job management)
    std::queue<std::pair<const uint8_t*, uint32_t>> print_data_queue_;

    // Configuration mappings (from Linux CUPS wrapper analysis)
    std::map<uint32_t, uint8_t> resolution_map_;
    std::map<const char*, uint8_t> pagesize_map_;
    std::map<const char*, uint8_t> mediatype_map_;
    std::map<const char*, uint8_t> papersource_map_;

    // Initialization
    kern_return_t InitializePrinterMaps();
};

#endif // BROTHERFAXDRIVER_MFP_EXTENDED_HPP
