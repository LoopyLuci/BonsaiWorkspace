import React, { useEffect } from 'react';
import { View, ActivityIndicator, StyleSheet } from 'react-native';
import { NavigationContainer } from '@react-navigation/native';
import { createStackNavigator } from '@react-navigation/stack';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';

import { AuthProvider, AuthContext } from './context/AuthContext';
import { AppProvider } from './context/AppContext';
import { SyncProvider } from './context/SyncContext';

import { AuthScreen } from './components/screens/AuthScreen';
import { HomeScreen } from './components/screens/HomeScreen';
import { BrowseScreen } from './components/screens/BrowseScreen';
import { RootStackParamList, MainTabParamList } from './types';

const Stack = createStackNavigator<RootStackParamList>();
const Tab = createBottomTabNavigator<MainTabParamList>();

// Auth Stack for unauthenticated users
const AuthStackNavigator = () => {
  return (
    <Stack.Navigator
      screenOptions={{
        headerShown: false,
      }}
    >
      <Stack.Screen name="Auth" component={AuthScreen} />
    </Stack.Navigator>
  );
};

// Main Tab Navigator for authenticated users
const MainTabNavigator = () => {
  return (
    <Tab.Navigator
      screenOptions={{
        headerShown: true,
        tabBarStyle: styles.tabBar,
        tabBarActiveTintColor: '#2563eb',
        tabBarInactiveTintColor: '#666',
        headerStyle: styles.header,
        headerTintColor: '#fff',
        headerTitleStyle: styles.headerTitle,
      }}
    >
      <Tab.Screen
        name="Home"
        component={HomeScreen}
        options={{
          title: 'Home',
          tabBarIcon: ({ color }) => (
            <View style={{ fontSize: 20 }}>🏠</View>
          ),
        }}
      />
      <Tab.Screen
        name="Browse"
        component={BrowseScreen}
        options={{
          title: 'Browse',
          tabBarIcon: ({ color }) => (
            <View style={{ fontSize: 20 }}>🔍</View>
          ),
        }}
      />
      <Tab.Screen
        name="Favorites"
        component={HomeScreen} // Placeholder
        options={{
          title: 'Favorites',
          tabBarIcon: ({ color }) => (
            <View style={{ fontSize: 20 }}>❤️</View>
          ),
        }}
      />
      <Tab.Screen
        name="Account"
        component={HomeScreen} // Placeholder
        options={{
          title: 'Account',
          tabBarIcon: ({ color }) => (
            <View style={{ fontSize: 20 }}>👤</View>
          ),
        }}
      />
    </Tab.Navigator>
  );
};

// Root Navigator
const RootNavigator = ({ isAuthenticated }: { isAuthenticated: boolean }) => {
  return (
    <NavigationContainer>
      {isAuthenticated ? (
        <MainTabNavigator />
      ) : (
        <AuthStackNavigator />
      )}
    </NavigationContainer>
  );
};

// App Component
const AppContent = () => {
  const authContext = React.useContext(AuthContext);

  if (!authContext) {
    return (
      <View style={styles.loadingContainer}>
        <ActivityIndicator size="large" color="#2563eb" />
      </View>
    );
  }

  const { auth } = authContext;

  if (auth.loading) {
    return (
      <View style={styles.loadingContainer}>
        <ActivityIndicator size="large" color="#2563eb" />
      </View>
    );
  }

  return <RootNavigator isAuthenticated={auth.isAuthenticated} />;
};

// Main App Component
export const App = () => {
  return (
    <AuthProvider>
      <AppProvider>
        <SyncProvider>
          <AppContent />
        </SyncProvider>
      </AppProvider>
    </AuthProvider>
  );
};

const styles = StyleSheet.create({
  loadingContainer: {
    flex: 1,
    backgroundColor: '#1a1a1a',
    justifyContent: 'center',
    alignItems: 'center',
  },
  tabBar: {
    backgroundColor: '#242424',
    borderTopColor: '#333',
    borderTopWidth: 1,
    height: 60,
  },
  header: {
    backgroundColor: '#242424',
    borderBottomColor: '#333',
    borderBottomWidth: 1,
  },
  headerTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: '#fff',
  },
});
