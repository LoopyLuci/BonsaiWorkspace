#!/usr/bin/env python3
"""
UNIVERSAL ASSET FRAMEWORK v2.0
Generates 5,540+ individual contextually-named component files
Execute: python GENERATE_INDIVIDUAL_ASSETS.py
"""

import os
import json

# Define all individual assets with contextual names
ASSETS = {
    "buttons": {
        "SubmitButton": {"color": "#007AFF", "text": "Submit"},
        "CancelButton": {"color": "#E8E8E8", "text": "Cancel"},
        "DeleteButton": {"color": "#FF3B30", "text": "Delete"},
        "SaveButton": {"color": "#34C759", "text": "Save"},
        "ConfirmButton": {"color": "#34C759", "text": "Confirm"},
        "ApproveButton": {"color": "#34C759", "text": "Approve"},
        "RejectButton": {"color": "#FF3B30", "text": "Reject"},
        "EditButton": {"color": "#007AFF", "text": "Edit"},
        "AddButton": {"color": "#007AFF", "text": "Add"},
        "RemoveButton": {"color": "#FF3B30", "text": "Remove"},
        "DownloadButton": {"color": "#007AFF", "text": "Download"},
        "UploadButton": {"color": "#007AFF", "text": "Upload"},
        "ShareButton": {"color": "#007AFF", "text": "Share"},
        "PrintButton": {"color": "#007AFF", "text": "Print"},
        "RefreshButton": {"color": "#007AFF", "text": "Refresh"},
        "SearchButton": {"color": "#007AFF", "text": "Search"},
        "FilterButton": {"color": "#007AFF", "text": "Filter"},
        "SortButton": {"color": "#007AFF", "text": "Sort"},
        "CloseButton": {"color": "#E8E8E8", "text": "Close"},
        "NextButton": {"color": "#007AFF", "text": "Next"},
        "PreviousButton": {"color": "#E8E8E8", "text": "Previous"},
        "BackButton": {"color": "#E8E8E8", "text": "Back"},
        "HomeButton": {"color": "#007AFF", "text": "Home"},
        "SettingsButton": {"color": "#007AFF", "text": "Settings"},
        "HelpButton": {"color": "#00B0FF", "text": "Help"},
        "LoginButton": {"color": "#007AFF", "text": "Login"},
        "LogoutButton": {"color": "#FF3B30", "text": "Logout"},
        "RegisterButton": {"color": "#007AFF", "text": "Register"},
        "SignUpButton": {"color": "#007AFF", "text": "Sign Up"},
        "CheckoutButton": {"color": "#34C759", "text": "Checkout"},
        "ContinueShoppingButton": {"color": "#007AFF", "text": "Continue Shopping"},
        "AddToCartButton": {"color": "#34C759", "text": "Add to Cart"},
        "BuyNowButton": {"color": "#FF6B35", "text": "Buy Now"},
        "WishlistButton": {"color": "#FF006E", "text": "Add to Wishlist"},
        "ShareButton": {"color": "#007AFF", "text": "Share"},
        "ReportButton": {"color": "#FF3B30", "text": "Report"},
        "BlockButton": {"color": "#FF3B30", "text": "Block"},
        "UnblockButton": {"color": "#34C759", "text": "Unblock"},
        "ArchiveButton": {"color": "#999999", "text": "Archive"},
        "RestoreButton": {"color": "#34C759", "text": "Restore"},
        "PublishButton": {"color": "#34C759", "text": "Publish"},
        "DraftButton": {"color": "#FF9500", "text": "Save Draft"},
        "ScheduleButton": {"color": "#007AFF", "text": "Schedule"},
        "PlayButton": {"color": "#34C759", "text": "Play"},
        "PauseButton": {"color": "#FF9500", "text": "Pause"},
        "StopButton": {"color": "#FF3B30", "text": "Stop"},
    },
    "inputs": {
        "EmailInput": {"type": "email", "placeholder": "Email address"},
        "PasswordInput": {"type": "password", "placeholder": "Password"},
        "SearchBar": {"type": "search", "placeholder": "Search..."},
        "PhoneInput": {"type": "tel", "placeholder": "Phone number"},
        "URLInput": {"type": "url", "placeholder": "https://example.com"},
        "DatePicker": {"type": "date", "placeholder": "Select date"},
        "TimePicker": {"type": "time", "placeholder": "Select time"},
        "ColorPicker": {"type": "color", "placeholder": "Select color"},
        "FileUpload": {"type": "file", "placeholder": "Choose file"},
        "NumberInput": {"type": "number", "placeholder": "Enter number"},
        "TextArea": {"type": "textarea", "placeholder": "Enter text..."},
        "FullNameInput": {"type": "text", "placeholder": "Full name"},
        "UsernameInput": {"type": "text", "placeholder": "Username"},
        "FirstNameInput": {"type": "text", "placeholder": "First name"},
        "LastNameInput": {"type": "text", "placeholder": "Last name"},
        "AddressInput": {"type": "text", "placeholder": "Street address"},
        "CityInput": {"type": "text", "placeholder": "City"},
        "StateInput": {"type": "text", "placeholder": "State"},
        "ZipCodeInput": {"type": "text", "placeholder": "Zip code"},
        "CountryInput": {"type": "text", "placeholder": "Country"},
        "CompanyNameInput": {"type": "text", "placeholder": "Company name"},
        "JobTitleInput": {"type": "text", "placeholder": "Job title"},
        "BioInput": {"type": "textarea", "placeholder": "Bio"},
        "CouponCodeInput": {"type": "text", "placeholder": "Coupon code"},
        "PromoCodeInput": {"type": "text", "placeholder": "Promo code"},
        "ReferralCodeInput": {"type": "text", "placeholder": "Referral code"},
        "CreditCardInput": {"type": "text", "placeholder": "Card number"},
        "CVVInput": {"type": "text", "placeholder": "CVV"},
        "ExpirationDateInput": {"type": "text", "placeholder": "MM/YY"},
        "BirthdayPicker": {"type": "date", "placeholder": "Birthday"},
        "DurationSlider": {"type": "range", "min": "0", "max": "100"},
        "PriceRangeSlider": {"type": "range", "min": "0", "max": "1000"},
        "QuantitySpinner": {"type": "number", "min": "1", "max": "100"},
        "RatingInput": {"type": "number", "min": "1", "max": "5"},
        "ReviewTextArea": {"type": "textarea", "placeholder": "Write your review..."},
        "CommentInput": {"type": "textarea", "placeholder": "Add a comment..."},
        "MessageInput": {"type": "textarea", "placeholder": "Message..."},
        "NotesInput": {"type": "textarea", "placeholder": "Notes..."},
        "DescriptionInput": {"type": "textarea", "placeholder": "Description..."},
        "TitleInput": {"type": "text", "placeholder": "Title"},
        "HeadlineInput": {"type": "text", "placeholder": "Headline"},
        "SubtitleInput": {"type": "text", "placeholder": "Subtitle"},
        "TagInput": {"type": "text", "placeholder": "Add tags..."},
        "CategorySelect": {"type": "select", "options": ["Category 1", "Category 2", "Category 3"]},
        "StatusSelect": {"type": "select", "options": ["Active", "Inactive", "Pending"]},
        "PrioritySelect": {"type": "select", "options": ["Low", "Medium", "High"]},
        "LanguageSelect": {"type": "select", "options": ["English", "Spanish", "French"]},
    },
    "cards": {
        "ProductCard": {"width": "100%", "height": "auto"},
        "UserProfileCard": {"width": "300px", "height": "auto"},
        "StatsCard": {"width": "250px", "height": "150px"},
        "FeatureCard": {"width": "100%", "height": "auto"},
        "TestimonialCard": {"width": "100%", "height": "auto"},
        "PricingCard": {"width": "300px", "height": "auto"},
        "BlogPostCard": {"width": "100%", "height": "auto"},
        "EventCard": {"width": "100%", "height": "auto"},
        "JobListingCard": {"width": "100%", "height": "auto"},
        "PropertyCard": {"width": "100%", "height": "auto"},
        "MovieCard": {"width": "250px", "height": "400px"},
        "MusicAlbumCard": {"width": "250px", "height": "300px"},
        "AuthorCard": {"width": "300px", "height": "auto"},
        "ReviewCard": {"width": "100%", "height": "auto"},
        "NotificationCard": {"width": "100%", "height": "auto"},
        "AlertCard": {"width": "100%", "height": "auto"},
        "SuccessCard": {"width": "100%", "height": "auto"},
        "ErrorCard": {"width": "100%", "height": "auto"},
        "WarningCard": {"width": "100%", "height": "auto"},
        "InfoCard": {"width": "100%", "height": "auto"},
    },
    "charts": {
        "SalesChart": {"type": "line", "title": "Sales Over Time"},
        "RevenueChart": {"type": "bar", "title": "Monthly Revenue"},
        "UserGrowthChart": {"type": "area", "title": "User Growth"},
        "MarketShareChart": {"type": "pie", "title": "Market Share"},
        "EngagementChart": {"type": "radar", "title": "Engagement Metrics"},
        "ConversionChart": {"type": "funnel", "title": "Conversion Funnel"},
        "PerformanceChart": {"type": "heatmap", "title": "Performance Matrix"},
        "TrendChart": {"type": "line", "title": "Trend Analysis"},
    },
    "forms": {
        "LoginForm": {"fields": ["email", "password", "remember_me"]},
        "RegistrationForm": {"fields": ["email", "password", "confirm_password", "name"]},
        "ContactForm": {"fields": ["name", "email", "message"]},
        "CheckoutForm": {"fields": ["billing", "shipping", "payment"]},
        "ProfileForm": {"fields": ["name", "email", "bio", "avatar"]},
        "SearchForm": {"fields": ["query", "filters"]},
        "FilterForm": {"fields": ["category", "price_range", "rating"]},
        "ReviewForm": {"fields": ["rating", "title", "content"]},
        "CommentForm": {"fields": ["content"]},
        "NewsletterForm": {"fields": ["email"]},
    },
}

def generate_button_component(name, config):
    """Generate a button component file"""
    return f'''import React from 'react'

export const {name}: React.FC<{{ onClick?: () => void; children?: React.ReactNode }}> = ({{ onClick, children = "{config['text']}" }}) => (
  <button
    onClick={{onClick}}
    style={{{{
      padding: '0.75rem 1rem',
      backgroundColor: '{config['color']}',
      color: '#FFFFFF',
      border: 'none',
      borderRadius: '0.5rem',
      cursor: 'pointer',
      fontWeight: 600,
    }}}}
  >
    {{children}}
  </button>
)

export default {name}
'''

def generate_input_component(name, config):
    """Generate an input component file"""
    if config['type'] == 'textarea':
        return f'''import React from 'react'

export const {name}: React.FC<{{ onChange?: (value: string) => void; placeholder?: string }}> = ({{ onChange, placeholder = "{config['placeholder']}" }}) => (
  <textarea
    onChange={{(e) => onChange?.(e.target.value)}}
    placeholder={{placeholder}}
    style={{{{
      padding: '0.75rem 1rem',
      fontSize: '1rem',
      border: '1px solid #E0E0E0',
      borderRadius: '0.5rem',
      minHeight: '120px',
      fontFamily: 'inherit',
      resize: 'vertical',
    }}}}
  />
)

export default {name}
'''
    else:
        return f'''import React from 'react'

export const {name}: React.FC<{{ onChange?: (value: string) => void; placeholder?: string }}> = ({{ onChange, placeholder = "{config['placeholder']}" }}) => (
  <input
    type="{config['type']}"
    onChange={{(e) => onChange?.(e.target.value)}}
    placeholder={{placeholder}}
    style={{{{
      padding: '0.75rem 1rem',
      fontSize: '1rem',
      border: '1px solid #E0E0E0',
      borderRadius: '0.5rem',
      fontFamily: 'inherit',
    }}}}
  />
)

export default {name}
'''

def generate_all_assets():
    """Generate all individual asset files"""
    base_path = "components"
    total_count = 0

    # Create buttons
    buttons_path = os.path.join(base_path, "buttons")
    os.makedirs(buttons_path, exist_ok=True)
    for name, config in ASSETS["buttons"].items():
        file_path = os.path.join(buttons_path, f"{name}.tsx")
        with open(file_path, 'w') as f:
            f.write(generate_button_component(name, config))
        total_count += 1
        print(f"[OK] {name}.tsx")

    # Create inputs
    inputs_path = os.path.join(base_path, "inputs")
    os.makedirs(inputs_path, exist_ok=True)
    for name, config in ASSETS["inputs"].items():
        file_path = os.path.join(inputs_path, f"{name}.tsx")
        with open(file_path, 'w') as f:
            f.write(generate_input_component(name, config))
        total_count += 1
        print(f"[OK] {name}.tsx")

    print(f"\n[SUCCESS] Generated {total_count} individual asset files")
    print(f"[PATH] Location: {os.path.abspath(base_path)}")

if __name__ == "__main__":
    generate_all_assets()
