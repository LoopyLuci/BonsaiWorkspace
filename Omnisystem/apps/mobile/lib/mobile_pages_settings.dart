// PATHFINDER Mobile - Settings Page
// User preferences, account settings, and app configuration

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/learning_provider.dart';

class SettingsPage extends StatefulWidget {
  const SettingsPage({Key? key}) : super(key: key);

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  bool _offlineMode = false;
  bool _pushNotifications = true;
  bool _emailNotifications = true;
  String _theme = 'light';
  String _language = 'en';
  String _emailFrequency = 'daily';

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _loadSettings();
    });
  }

  Future<void> _loadSettings() async {
    final provider = context.read<LearningProvider>();
    // Load user preferences from local storage or API
  }

  @override
  Widget build(BuildContext context) => SafeArea(
      child: SingleChildScrollView(
        child: Column(
          children: [
            // Header
            Container(
              padding: EdgeInsets.all(24),
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: [Color(0xFF4F46E5), Color(0xFF3B38CC)],
                ),
              ),
              child: Row(
                children: [
                  CircleAvatar(
                    radius: 30,
                    backgroundColor: Colors.white.withOpacity(0.3),
                    child: Icon(Icons.person, color: Colors.white, size: 32),
                  ),
                  SizedBox(width: 16),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Account',
                          style: TextStyle(
                            color: Colors.white,
                            fontSize: 20,
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                        SizedBox(height: 4),
                        Text(
                          'Manage your profile',
                          style: TextStyle(
                            color: Colors.white70,
                            fontSize: 12,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),

            // Account Settings Section
            _buildSection(
              'Account',
              [
                _buildListTile(
                  icon: Icons.edit,
                  title: 'Edit Profile',
                  subtitle: 'Change name, avatar, timezone',
                  onTap: () => _showEditProfile(),
                ),
                _buildListTile(
                  icon: Icons.lock,
                  title: 'Change Password',
                  subtitle: 'Update your password',
                  onTap: () => _showChangePassword(),
                ),
                _buildListTile(
                  icon: Icons.email,
                  title: 'Email Address',
                  subtitle: 'Manage your email',
                  onTap: () => _showChangeEmail(),
                ),
              ],
            ),

            // Notification Settings Section
            _buildSection(
              'Notifications',
              [
                _buildSwitchTile(
                  icon: Icons.notifications,
                  title: 'Push Notifications',
                  subtitle: 'Receive app notifications',
                  value: _pushNotifications,
                  onChanged: (val) {
                    setState(() => _pushNotifications = val);
                    _saveNotificationPreferences();
                  },
                ),
                _buildSwitchTile(
                  icon: Icons.mail,
                  title: 'Email Notifications',
                  subtitle: 'Receive email updates',
                  value: _emailNotifications,
                  onChanged: (val) {
                    setState(() => _emailNotifications = val);
                    _saveNotificationPreferences();
                  },
                ),
                _buildDropdownTile(
                  icon: Icons.schedule,
                  title: 'Email Frequency',
                  subtitle: 'How often to receive emails',
                  value: _emailFrequency,
                  options: ['immediate', 'daily', 'weekly', 'never'],
                  onChanged: (val) {
                    setState(() => _emailFrequency = val);
                    _saveNotificationPreferences();
                  },
                ),
              ],
            ),

            // Learning Settings Section
            _buildSection(
              'Learning',
              [
                _buildSwitchTile(
                  icon: Icons.cloud_off,
                  title: 'Offline Mode',
                  subtitle: 'Download exercises for offline use',
                  value: _offlineMode,
                  onChanged: (val) {
                    setState(() => _offlineMode = val);
                    if (val) {
                      _downloadExercises();
                    }
                  },
                ),
                _buildListTile(
                  icon: Icons.storage,
                  title: 'Storage',
                  subtitle: 'Manage downloaded content',
                  trailing: Text('2.3 GB'),
                  onTap: () => _showStorageManagement(),
                ),
                _buildListTile(
                  icon: Icons.analytics,
                  title: 'Learning Insights',
                  subtitle: 'View your learning analytics',
                  onTap: () => _showLearningInsights(),
                ),
              ],
            ),

            // App Settings Section
            _buildSection(
              'App',
              [
                _buildDropdownTile(
                  icon: Icons.color_lens,
                  title: 'Theme',
                  subtitle: 'Choose your preferred theme',
                  value: _theme,
                  options: ['light', 'dark', 'auto'],
                  onChanged: (val) {
                    setState(() => _theme = val);
                    // Apply theme
                  },
                ),
                _buildDropdownTile(
                  icon: Icons.language,
                  title: 'Language',
                  subtitle: 'Choose your language',
                  value: _language,
                  options: ['en', 'es', 'fr', 'de', 'ja', 'zh'],
                  onChanged: (val) {
                    setState(() => _language = val);
                    // Change language
                  },
                ),
                _buildListTile(
                  icon: Icons.info,
                  title: 'About',
                  subtitle: 'Version 1.0.0',
                  onTap: () => _showAbout(),
                ),
              ],
            ),

            // Data & Privacy Section
            _buildSection(
              'Data & Privacy',
              [
                _buildListTile(
                  icon: Icons.download,
                  title: 'Download My Data',
                  subtitle: 'Export your data (GDPR)',
                  onTap: () => _downloadData(),
                ),
                _buildListTile(
                  icon: Icons.delete_forever,
                  title: 'Delete Account',
                  subtitle: 'Permanently delete your account',
                  onTap: () => _confirmDeleteAccount(),
                  titleColor: Colors.red,
                ),
                _buildListTile(
                  icon: Icons.description,
                  title: 'Privacy Policy',
                  subtitle: 'Read our privacy policy',
                  onTap: () => _showPrivacyPolicy(),
                ),
                _buildListTile(
                  icon: Icons.assignment,
                  title: 'Terms of Service',
                  subtitle: 'Read our terms of service',
                  onTap: () => _showTermsOfService(),
                ),
              ],
            ),

            // Logout Button
            Padding(
              padding: EdgeInsets.all(16),
              child: SizedBox(
                width: double.infinity,
                child: ElevatedButton.icon(
                  onPressed: () => _confirmLogout(),
                  icon: Icon(Icons.logout),
                  label: Text('Logout'),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: Colors.red[400],
                    foregroundColor: Colors.white,
                    padding: EdgeInsets.symmetric(vertical: 12),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );

  Widget _buildSection(String title, List<Widget> children) => Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: EdgeInsets.fromLTRB(16, 16, 16, 8),
          child: Text(
            title,
            style: TextStyle(
              fontSize: 14,
              fontWeight: FontWeight.bold,
              color: Color(0xFF4F46E5),
            ),
          ),
        ),
        ...children,
        Divider(height: 1),
      ],
    );

  Widget _buildListTile({
    required IconData icon,
    required String title,
    required String subtitle,
    Widget? trailing,
    Color? titleColor,
    required VoidCallback onTap,
  }) => ListTile(
      leading: Icon(icon, color: Color(0xFF4F46E5)),
      title: Text(title, style: TextStyle(color: titleColor)),
      subtitle: Text(subtitle, style: TextStyle(fontSize: 12)),
      trailing: trailing ?? Icon(Icons.chevron_right, color: Colors.grey),
      onTap: onTap,
    );

  Widget _buildSwitchTile({
    required IconData icon,
    required String title,
    required String subtitle,
    required bool value,
    required Function(bool) onChanged,
  }) => ListTile(
      leading: Icon(icon, color: Color(0xFF4F46E5)),
      title: Text(title),
      subtitle: Text(subtitle, style: TextStyle(fontSize: 12)),
      trailing: Switch(
        value: value,
        onChanged: onChanged,
        activeColor: Color(0xFF4F46E5),
      ),
    );

  Widget _buildDropdownTile({
    required IconData icon,
    required String title,
    required String subtitle,
    required String value,
    required List<String> options,
    required Function(String) onChanged,
  }) => ListTile(
      leading: Icon(icon, color: Color(0xFF4F46E5)),
      title: Text(title),
      subtitle: Text(subtitle, style: TextStyle(fontSize: 12)),
      trailing: DropdownButton<String>(
        value: value,
        items: options.map((option) {
          return DropdownMenuItem(
            value: option,
            child: Text(option),
          );
        }).toList(),
        onChanged: (val) => val != null ? onChanged(val) : null,
        underline: SizedBox(),
      ),
    );

  void _showEditProfile() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Edit Profile'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(decoration: InputDecoration(hintText: 'Name')),
            SizedBox(height: 12),
            TextField(decoration: InputDecoration(hintText: 'Timezone')),
          ],
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context), child: Text('Cancel')),
          FilledButton(onPressed: () => Navigator.pop(context), child: Text('Save')),
        ],
      ),
    );
  }

  void _showChangePassword() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Change Password'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              decoration: InputDecoration(hintText: 'Current Password'),
              obscureText: true,
            ),
            SizedBox(height: 12),
            TextField(
              decoration: InputDecoration(hintText: 'New Password'),
              obscureText: true,
            ),
            SizedBox(height: 12),
            TextField(
              decoration: InputDecoration(hintText: 'Confirm Password'),
              obscureText: true,
            ),
          ],
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context), child: Text('Cancel')),
          FilledButton(onPressed: () => Navigator.pop(context), child: Text('Update')),
        ],
      ),
    );
  }

  void _showChangeEmail() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Change Email'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(decoration: InputDecoration(hintText: 'New Email')),
            SizedBox(height: 12),
            TextField(
              decoration: InputDecoration(hintText: 'Password'),
              obscureText: true,
            ),
          ],
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context), child: Text('Cancel')),
          FilledButton(onPressed: () => Navigator.pop(context), child: Text('Update')),
        ],
      ),
    );
  }

  void _showStorageManagement() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Storage Management'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Downloaded Exercises: 1.2 GB'),
            SizedBox(height: 8),
            Text('Cached Data: 0.8 GB'),
            SizedBox(height: 8),
            Text('Photos: 0.3 GB'),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: Text('Clear Cache'),
          ),
          FilledButton(onPressed: () => Navigator.pop(context), child: Text('Close')),
        ],
      ),
    );
  }

  void _showLearningInsights() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Learning Analytics'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Time Spent: 25 hours'),
            SizedBox(height: 8),
            Text('Exercises Completed: 152'),
            SizedBox(height: 8),
            Text('Accuracy: 82%'),
            SizedBox(height: 8),
            Text('Streak: 7 days'),
          ],
        ),
        actions: [
          FilledButton(onPressed: () => Navigator.pop(context), child: Text('Close')),
        ],
      ),
    );
  }

  void _showAbout() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('About PATHFINDER'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Version: 1.0.0'),
            SizedBox(height: 8),
            Text('Build: 2026.06.11'),
            SizedBox(height: 12),
            Text('PATHFINDER - The science-backed learning platform.'),
          ],
        ),
        actions: [
          FilledButton(onPressed: () => Navigator.pop(context), child: Text('Close')),
        ],
      ),
    );
  }

  void _showPrivacyPolicy() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Privacy Policy'),
        content: SingleChildScrollView(
          child: Text('Privacy policy content...'),
        ),
        actions: [
          FilledButton(onPressed: () => Navigator.pop(context), child: Text('Close')),
        ],
      ),
    );
  }

  void _showTermsOfService() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Terms of Service'),
        content: SingleChildScrollView(
          child: Text('Terms of service content...'),
        ),
        actions: [
          FilledButton(onPressed: () => Navigator.pop(context), child: Text('Close')),
        ],
      ),
    );
  }

  void _downloadData() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Download My Data'),
        content: Text('Your data export will be prepared and sent to your email.'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context), child: Text('Cancel')),
          FilledButton(
            onPressed: () {
              Navigator.pop(context);
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(content: Text('Data export requested. Check your email.')),
              );
            },
            child: Text('Export'),
          ),
        ],
      ),
    );
  }

  void _confirmDeleteAccount() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Delete Account'),
        content: Text('This action is permanent and cannot be undone. All your data will be deleted.'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context), child: Text('Cancel')),
          FilledButton(
            onPressed: () {
              Navigator.pop(context);
              _deleteAccount();
            },
            style: FilledButton.styleFrom(backgroundColor: Colors.red),
            child: Text('Delete'),
          ),
        ],
      ),
    );
  }

  void _confirmLogout() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Logout'),
        content: Text('Are you sure you want to logout?'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context), child: Text('Cancel')),
          FilledButton(
            onPressed: () {
              Navigator.pop(context);
              context.read<LearningProvider>().logout();
              Navigator.pushReplacementNamed(context, '/login');
            },
            child: Text('Logout'),
          ),
        ],
      ),
    );
  }

  void _saveNotificationPreferences() {
    // Save to API/local storage
  }

  void _downloadExercises() {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('Downloading exercises for offline use...')),
    );
  }

  void _deleteAccount() {
    context.read<LearningProvider>().deleteAccount();
    Navigator.pushReplacementNamed(context, '/login');
  }
}
