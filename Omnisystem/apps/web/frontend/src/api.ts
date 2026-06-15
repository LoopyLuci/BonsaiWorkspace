import axios from 'axios'
import { useAuthStore } from './stores/authStore'

const API_BASE_URL = process.env.REACT_APP_API_URL || 'http://localhost:8000'

const apiClient = axios.create({
  baseURL: `${API_BASE_URL}/api/v1`,
})

apiClient.interceptors.request.use((config) => {
  const token = useAuthStore.getState().token
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

export const authAPI = {
  register: (email: string, password: string, name: string) =>
    apiClient.post('/auth/register', { email, password, name }),
  login: (email: string, password: string) =>
    apiClient.post('/auth/login', { email, password }),
}

export const skillsAPI = {
  list: () => apiClient.get('/skills'),
  get: (id: string) => apiClient.get(`/skills/${id}`),
  create: (data: any) => apiClient.post('/skills', data),
}

export const exercisesAPI = {
  list: () => apiClient.get('/exercises'),
  get: (id: string) => apiClient.get(`/exercises/${id}`),
  create: (data: any) => apiClient.post('/exercises', data),
  submitAttempt: (data: any) => apiClient.post('/exercises/attempts', data),
}

export const progressAPI = {
  list: (userId: string) => apiClient.get(`/progress/user/${userId}`),
  get: (userId: string, skillId: string) =>
    apiClient.get(`/progress/user/${userId}/skill/${skillId}`),
}

export const classroomsAPI = {
  list: () => apiClient.get('/classrooms'),
  create: (data: any) => apiClient.post('/classrooms', data),
}

export const notificationsAPI = {
  list: (userId: string) => apiClient.get(`/notifications/user/${userId}`),
}

export const achievementsAPI = {
  list: (userId: string) => apiClient.get(`/achievements/user/${userId}`),
}

export const searchAPI = {
  search: (query: string) => apiClient.get('/search', { params: { query } }),
}
