import { useQuery } from '@tanstack/react-query'
import { useAuthStore } from '../stores/authStore'
import axios from 'axios'

export default function RecommendationsPage() {
  const { user } = useAuthStore()
  const { data: recommendations, isLoading } = useQuery({
    queryKey: ['recommendations', user?.id],
    queryFn: () =>
      axios.get(`/api/v1/personalization/user/${user?.id}/recommendations`),
    enabled: !!user?.id,
  })

  if (isLoading) return <div className="p-8">Loading...</div>

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="container mx-auto">
        <h1 className="text-3xl font-bold mb-8">Recommended for You</h1>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {recommendations?.data?.recommendations?.map((rec: any) => (
            <div key={rec.skill_id} className="bg-white rounded-lg shadow p-6">
              <div className="flex justify-between items-start mb-4">
                <h3 className="text-lg font-bold">{rec.skill_id}</h3>
                <span className="bg-blue-100 text-blue-800 px-3 py-1 rounded-full text-sm">
                  {Math.round(rec.confidence * 100)}%
                </span>
              </div>
              <p className="text-gray-600 mb-4">{rec.reason}</p>
              <button className="w-full bg-blue-600 hover:bg-blue-700 text-white py-2 rounded">
                Start Learning
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
