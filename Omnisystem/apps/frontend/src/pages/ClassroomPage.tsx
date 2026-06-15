import { useQuery } from '@tanstack/react-query'
import { classroomsAPI } from '../api'

export default function ClassroomPage() {
  const { data: classrooms, isLoading } = useQuery({
    queryKey: ['classrooms'],
    queryFn: () => classroomsAPI.list(),
  })

  if (isLoading) return <div className="p-8">Loading...</div>

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="container mx-auto">
        <h1 className="text-3xl font-bold mb-8">Classrooms</h1>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {classrooms?.data?.map((classroom: any) => (
            <div key={classroom.id} className="bg-white rounded-lg shadow p-6">
              <h3 className="text-xl font-bold mb-2">{classroom.name}</h3>
              <p className="text-gray-600">Grade: {classroom.grade_level}</p>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
