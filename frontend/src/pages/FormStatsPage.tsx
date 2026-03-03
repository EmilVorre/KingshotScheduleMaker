import { useEffect, useState, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router-dom'
import { api, type FormStats } from '../api/client'
import { LANGUAGE_OPTIONS, type SupportedLanguage } from '../i18n'

function parseTimeToMinutes(timeStr: string): number {
  const parts = timeStr.split(':')
  if (parts.length !== 2) return 0
  const hours = parseInt(parts[0], 10) || 0
  const minutes = parseInt(parts[1], 10) || 0
  return hours * 60 + minutes
}

function sortTimeSlots(
  timeSlotMap: Record<string, { requests: number }> | undefined,
  startTime: string
): Record<string, { requests: number }> {
  if (!timeSlotMap) return {}
  const entries = Object.entries(timeSlotMap)
  const startMinutes = parseTimeToMinutes(startTime)
  entries.sort((a, b) => {
    const minutesA = parseTimeToMinutes(a[0])
    const minutesB = parseTimeToMinutes(b[0])
    const adjustedA = minutesA < startMinutes ? minutesA + 24 * 60 : minutesA
    const adjustedB = minutesB < startMinutes ? minutesB + 24 * 60 : minutesB
    return adjustedA - adjustedB
  })
  return Object.fromEntries(entries)
}

function TimeSlotGrid({
  slots,
  titleColor,
  hoverBorder,
  icon,
  dayLabel,
  t,
}: {
  slots: Record<string, { requests: number }>
  titleColor: string
  hoverBorder: string
  icon: string
  dayLabel: string
  t: (key: string, opts?: { day?: string }) => string
}) {
  return (
    <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
      <h2 className={`text-2xl font-bold ${titleColor} mb-6`}>
        <i className={`fas fa-${icon} mr-2`}></i>
        {t('dayTimeSlotPopularity', { day: dayLabel })}
      </h2>
      <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 gap-2">
        {Object.entries(slots).map(([time, timeData]) => (
          <div
            key={time}
            className={`bg-gray-700/50 rounded-lg p-3 border border-gray-600 hover:shadow-md transition-all text-center ${hoverBorder}`}
          >
            <div className={`font-bold ${titleColor} text-sm mb-1`}>{time}</div>
            <div className="text-xl font-bold text-white">{timeData.requests}</div>
          </div>
        ))}
      </div>
    </div>
  )
}

export default function FormStatsPage() {
  const { code } = useParams<{ code: string }>()
  const { t, i18n } = useTranslation()
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [stats, setStats] = useState<FormStats | null>(null)

  const baseUrl = `/form/${code}`

  const sortedConstruction = useMemo(
    () => sortTimeSlots(stats?.construction_time_slot_popularity, stats?.construction_start_time || '00:00'),
    [stats]
  )
  const sortedResearch = useMemo(
    () => sortTimeSlots(stats?.research_time_slot_popularity, stats?.research_start_time || '00:00'),
    [stats]
  )
  const sortedTroops = useMemo(
    () => sortTimeSlots(stats?.troops_time_slot_popularity, stats?.troops_start_time || '00:00'),
    [stats]
  )

  useEffect(() => {
    if (!code) return
    setLoading(true)
    setError(null)
    api.getFormStats(code).then(({ ok, data, error: err }) => {
      if (ok && data) setStats(data)
      else setError(err || i18n.t('failedToLoadStatistics'))
      setLoading(false)
    })
  }, [code])

  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      <header className="text-center mb-12">
        <h1 className="text-4xl font-bold text-blue-400 mb-4">
          <i className="fas fa-chart-bar mr-3"></i>{t('formStatistics')}
        </h1>
        <p className="text-gray-400">{t('formStatisticsDescription')}</p>
      </header>

      <div className="flex justify-end mb-4">
        <select
          value={i18n.language}
          onChange={(e) => i18n.changeLanguage(e.target.value as SupportedLanguage)}
          className="px-4 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
        >
          {LANGUAGE_OPTIONS.map((o) => (
            <option key={o.value} value={o.value}>{o.label}</option>
          ))}
        </select>
      </div>

      <nav className="flex justify-center gap-4 mb-12 flex-wrap">
        <Link
          to={`${baseUrl}/stats`}
          className="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-all border border-blue-500"
        >
          <i className="fas fa-chart-bar mr-2"></i>{t('statistics')}
        </Link>
        <Link
          to={baseUrl}
          className="px-6 py-3 bg-gray-800 hover:bg-gray-700 text-white rounded-lg font-medium transition-all border border-gray-700"
        >
          <i className="fas fa-edit mr-2"></i>{t('submitForm')}
        </Link>
      </nav>

      <main>
        {loading && (
          <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
            <i className="fas fa-spinner fa-spin text-4xl text-blue-400 mb-4"></i>
            <p className="text-xl text-gray-400">{t('loadingStatistics')}</p>
          </div>
        )}
        {error && (
          <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-red-700">
            <div className="text-center">
              <i className="fas fa-exclamation-triangle text-4xl text-red-400 mb-4"></i>
              <h2 className="text-2xl font-bold text-red-400 mb-2">{t('error')}</h2>
              <p className="text-gray-300">{error}</p>
            </div>
          </div>
        )}
        {!loading && !error && stats && (
          <div className="space-y-8">
            <TimeSlotGrid
              slots={sortedConstruction}
              titleColor="text-orange-400"
              hoverBorder="hover:border-orange-500"
              icon="hammer"
              dayLabel={t('constructionDay')}
              t={t}
            />
            <TimeSlotGrid
              slots={sortedResearch}
              titleColor="text-purple-400"
              hoverBorder="hover:border-purple-500"
              icon="flask"
              dayLabel={t('researchDay')}
              t={t}
            />
            <TimeSlotGrid
              slots={sortedTroops}
              titleColor="text-green-400"
              hoverBorder="hover:border-green-500"
              icon="users"
              dayLabel={t('troopsTrainingDay')}
              t={t}
            />
          </div>
        )}
      </main>
    </div>
  )
}
