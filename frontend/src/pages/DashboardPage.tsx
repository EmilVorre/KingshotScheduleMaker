import { useCallback, useEffect, useRef, useState } from 'react'
import { Link, useNavigate, useParams, useSearchParams } from 'react-router-dom'
import { useAuth } from '../context/AuthContext'
import {
  api,
  type AccountStats,
  type CreateFormRequest,
  type CurrentFormInfo,
  type PredeterminedSlot,
  type Schedule,
} from '../api/client'

const STANDARD_INTRO_TEXT =
  'Fill out this form to apply for Chief Minister (CM) and Noble Advisor (NA) appointments.\n\nSchedule:\n- Construction Day (Monday) [CM]\n- Research Day (Tuesday) [CM]\n- Troops Training Day (Thursday) [NA]\n\nRequirements:\n\n- Form must be filled out in order to be considered for an appointment during SvS preparation week. \n- Form must be filled out by THE SUNDAY OF MATCHMAKING.\n- Form filled out after the deadline will be added to the "Late" submission wait list.\n- Rally leaders and rally leader substitutes may be given priority (if necessary).\n- Verification of items, speedups, and resources may be requested (eg. during situations where the score is very close in points and to make sure our state wins by ensuring appointments go to players who can maximize points).\n\n\nFor more information:\n- Contact form support: #140 [COB]Vor and /or the current Minister of Justice if you have questions on filling out this form or changes to your form submission!'

const SUBMISSION_HEADERS = [
  'Timestamp',
  'Name',
  'Alliance',
  'Construction speedups',
  'Truegold',
  'want Construction?',
  'Construction times',
  'Research Speedups',
  'Truegold Dust',
  'want Research?',
  'Research times',
  'Troop Speedups',
  'Want troops?',
  'Troop times',
]

const SCHEDULE_DAYS = {
  construction: {
    name: 'Construction Day',
    icon: 'fas fa-hammer',
    buttonClass: 'bg-orange-600 hover:bg-orange-700 text-white',
    ringClass: 'ring-4 ring-orange-400',
  },
  research: {
    name: 'Research Day',
    icon: 'fas fa-flask',
    buttonClass: 'bg-blue-600 hover:bg-blue-700 text-white',
    ringClass: 'ring-4 ring-blue-400',
  },
  troops: {
    name: 'Troops Training Day',
    icon: 'fas fa-users',
    buttonClass: 'bg-green-600 hover:bg-green-700 text-white',
    ringClass: 'ring-4 ring-green-400',
  },
} as const

type ScheduleDayKey = keyof typeof SCHEDULE_DAYS
type Tab = 'profile' | 'schedule' | 'stats' | 'create-form' | 'current-form' | 'csv-operations' | 'generate-schedule'

interface ExtendedPredeterminedSlot extends PredeterminedSlot {
  lookingUp?: boolean
  lookupError?: string | null
}

function parseTimeToMinutes(timeStr: string): number {
  const parts = timeStr.split(':')
  if (parts.length !== 2) return 0
  const hours = parseInt(parts[0], 10) || 0
  const minutes = parseInt(parts[1], 10) || 0
  return hours * 60 + minutes
}

function sortTimeSlots(
  timeSlotMap: Record<string, { requests: number }>,
  startTime: string
): Record<string, { requests: number }> {
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

function getSubmissionValue(submission: Record<string, unknown>, header: string): string {
  if (!submission || typeof submission !== 'object') return ''
  const keys = Object.keys(submission)
  const findKey = (patterns: string[]) => {
    for (const pattern of patterns) {
      const normalizedPattern = pattern.toLowerCase().replace(/\s+/g, ' ').trim()
      const found = keys.find((k) => {
        const normalizedKey = k
          .toLowerCase()
          .replace(/\s+/g, ' ')
          .replace(/\n/g, ' ')
          .trim()
        return normalizedKey.includes(normalizedPattern)
      })
      if (found) return found
    }
    return null
  }
  const columnMap: Record<string, string | null> = {
    Timestamp: 'timestamp',
    Name: findKey(['character name']),
    Alliance: findKey(['alliance do you belong']),
    'Construction speedups': findKey([
      'speedups do you plan to use on construction day',
      'construction day',
      'speedups',
      'construction',
    ]),
    Truegold: findKey(['how much truegold do you plan', 'truegold', 'plan to', 'spend']),
    'want Construction?': findKey(['do you want a construction day appointment']),
    'Construction times': findKey([
      'times are you available for your construction day appointment',
      'construction day appointment',
      'utc time',
      'construction',
    ]),
    'Research Speedups': findKey([
      'speedups do you plan to use on research day',
      'research day',
      'speedups',
      'research',
    ]),
    'Truegold Dust': findKey(['how much truegold dust do you plan', 'truegold dust']),
    'want Research?': findKey(['do you want a research day appointment']),
    'Research times': findKey([
      'times are you available for your research day appointment',
      'research day appointment',
      'utc time',
      'research',
    ]),
    'Troop Speedups': findKey([
      'speedups do you plan to use on troops training day',
      'troops training day',
      'speedups',
      'troops',
    ]),
    'Want troops?': findKey(['do you want a troops training day appointment']),
    'Troop times': findKey([
      'times are you available for your troops training day appointment',
      'troops training day appointment',
      'utc time',
      'troops',
    ]),
  }
  const columnKey = columnMap[header]
  if (columnKey && submission[columnKey] !== undefined) {
    return String(submission[columnKey])
  }
  return String(submission[header] ?? '')
}

const TAB_KEYS: Tab[] = ['profile', 'schedule', 'stats', 'create-form', 'current-form', 'csv-operations', 'generate-schedule']

export default function DashboardPage() {
  const { accountName } = useParams<{ accountName: string }>()
  const { refresh: refreshAuth } = useAuth()
  const navigate = useNavigate()
  const [searchParams, setSearchParams] = useSearchParams()
  const tabParam = searchParams.get('tab') as Tab | null
  const activeTab: Tab = tabParam && TAB_KEYS.includes(tabParam) ? tabParam : 'schedule'

  function selectTab(tab: Tab) {
    setSearchParams({ tab })
  }
  const [sessionValid, setSessionValid] = useState<boolean | null>(null)
  const [serverNumber, setServerNumber] = useState<number | null>(null)
  const [playerId, setPlayerId] = useState<string | null>(null)
  const [inGameName, setInGameName] = useState<string | null>(null)

  // Profile edit state
  const [profileEdit, setProfileEdit] = useState({ account_name: '', server_number: '', in_game_name: '' })
  const [profileSaving, setProfileSaving] = useState(false)
  const [profileError, setProfileError] = useState<string | null>(null)
  const [kingshotIdInput, setKingshotIdInput] = useState('')
  const [kingshotLookingUp, setKingshotLookingUp] = useState(false)
  const [kingshotError, setKingshotError] = useState<string | null>(null)

  // Schedule tab
  const [currentScheduleDay, setCurrentScheduleDay] = useState<ScheduleDayKey>('construction')
  const [scheduleLoading, setScheduleLoading] = useState(false)
  const [scheduleError, setScheduleError] = useState<string | null>(null)
  const [currentSchedule, setCurrentSchedule] = useState<Schedule | null>(null)
  const [editingSlot, setEditingSlot] = useState<{ time: string; player: string } | null>(null)
  const [savingSlot, setSavingSlot] = useState(false)
  const slotInputRef = useRef<HTMLInputElement>(null)

  // Stats tab
  const [statsLoading, setStatsLoading] = useState(false)
  const [statsError, setStatsError] = useState<string | null>(null)
  const [stats, setStats] = useState<AccountStats | null>(null)

  // Create Form tab
  const [config, setConfig] = useState({
    form_name: '',
    kingdom_id: '',
    support_person_name: '',
    alliances: [] as string[],
    include_non_of_above: true,
    construction_truegold_mode: 'truegold_unlocked' as string,
  })
  const [creatingForm, setCreatingForm] = useState(false)
  const [configStatus, setConfigStatus] = useState<{ type: 'success' | 'error'; message: string } | null>(null)
  const [createdFormUrl, setCreatedFormUrl] = useState<string | null>(null)
  const [createdFormCode, setCreatedFormCode] = useState<string | null>(null)

  // Current Form tab
  const [currentForm, setCurrentForm] = useState<CurrentFormInfo | null>(null)
  const [loadingCurrentForm, setLoadingCurrentForm] = useState(false)
  const [submissions, setSubmissions] = useState<Record<string, unknown>[] | null>(null)
  const [loadingSubmissions, setLoadingSubmissions] = useState(false)
  const [submissionsError, setSubmissionsError] = useState<string | null>(null)
  const [oldForms, setOldForms] = useState<Array<{ archive_name: string; code: string; name: string; created_at: string; delete_date?: string }>>([])
  const [loadingOldForms, setLoadingOldForms] = useState(false)
  const [reopeningForm, setReopeningForm] = useState(false)
  const [clearScheduleLoading, setClearScheduleLoading] = useState<string | null>(null)

  // CSV Operations tab
  const [selectedFile, setSelectedFile] = useState<File | null>(null)
  const [uploading, setUploading] = useState(false)
  const [uploadStatus, setUploadStatus] = useState<{ type: 'success' | 'error'; message: string } | null>(null)
  const [downloadingCSV, setDownloadingCSV] = useState(false)

  // Generate Schedule tab
  const [predeterminedSlots, setPredeterminedSlots] = useState<ExtendedPredeterminedSlot[]>([])
  const [generatingSchedule, setGeneratingSchedule] = useState(false)
  const [scheduleGenStatus, setScheduleGenStatus] = useState<{
    type: 'success' | 'error'
    message: string
  } | null>(null)

  const loadSession = useCallback(async () => {
    const { ok, data } = await api.getSession()
    if (!ok || !data?.account_name || data.account_name !== accountName) {
      setSessionValid(false)
      navigate('/', { replace: true })
    } else {
      setSessionValid(true)
      setServerNumber(data.server_number ?? null)
      setPlayerId(data.player_id ?? null)
      setInGameName(data.in_game_name ?? null)
      setProfileEdit({
        account_name: data.account_name ?? '',
        server_number: String(data.server_number ?? ''),
        in_game_name: data.in_game_name ?? '',
      })
    }
  }, [accountName, navigate])

  useEffect(() => {
    loadSession()
  }, [loadSession])

  const loadSchedule = useCallback(
    async (day: ScheduleDayKey) => {
      if (!accountName || !serverNumber) return
      setScheduleLoading(true)
      setScheduleError(null)
      setCurrentSchedule(null)
      setEditingSlot(null)
      const { ok, data, error } = await api.getSchedule(accountName, serverNumber, day)
      setScheduleLoading(false)
      if (ok && data) setCurrentSchedule(data)
      else setScheduleError(error ?? 'Failed to load schedule')
    },
    [accountName, serverNumber]
  )

  const loadStats = useCallback(async () => {
    if (!accountName || !serverNumber) return
    setStatsLoading(true)
    setStatsError(null)
    const { ok, data, error } = await api.getStats(accountName, serverNumber)
    setStatsLoading(false)
    if (ok && data) setStats(data)
    else setStatsError(error ?? 'Failed to load statistics')
  }, [accountName, serverNumber])

  const loadCurrentForm = useCallback(async () => {
    if (!accountName || !serverNumber) return
    setLoadingCurrentForm(true)
    const { ok, data } = await api.getCurrentForm(accountName, serverNumber)
    setLoadingCurrentForm(false)
    const form = (data as { form?: CurrentFormInfo })?.form ?? data
    if (ok && form) {
      setCurrentForm(form as CurrentFormInfo)
      const configData = (data as { form?: { config?: { predetermined_slots?: PredeterminedSlot[] } } })?.form
        ?.config
      if (configData?.predetermined_slots) {
        setPredeterminedSlots(
          configData.predetermined_slots.map((s) => ({
            ...s,
            lookingUp: false,
            lookupError: null,
          }))
        )
      } else {
        setPredeterminedSlots([])
      }
    } else {
      setCurrentForm(null)
      setPredeterminedSlots([])
      setSubmissions(null)
    }
  }, [accountName, serverNumber])

  const loadSubmissions = useCallback(async () => {
    if (!accountName || !serverNumber || !currentForm) return
    setLoadingSubmissions(true)
    setSubmissionsError(null)
    const { ok, data, error } = await api.getFormSubmissions(accountName, serverNumber)
    setLoadingSubmissions(false)
    const subs = (data as { submissions?: Record<string, unknown>[] })?.submissions ?? []
    if (ok) setSubmissions(subs)
    else {
      setSubmissionsError(error ?? 'Failed to load submissions')
      setSubmissions([])
    }
  }, [accountName, serverNumber, currentForm])

  const loadConfig = useCallback(async () => {
    if (!accountName || !serverNumber) return
    const { ok, data } = await api.getPreviousFormConfig(accountName, serverNumber)
    if (ok && data?.config) {
      const c = data.config
      setConfig({
        form_name: '',
        kingdom_id: (c as { kingdom_id?: string }).kingdom_id ?? '',
        support_person_name: (c as { support_person_name?: string }).support_person_name ?? '',
        alliances: ((c.alliances ?? []) as string[]).filter((a) => a !== 'Non of the above'),
        include_non_of_above: (c as { include_non_of_above?: boolean }).include_non_of_above !== false,
        construction_truegold_mode:
          (c as { construction_truegold_mode?: string }).construction_truegold_mode ?? 'truegold_unlocked',
      })
    }
  }, [accountName, serverNumber])

  useEffect(() => {
    if (sessionValid && accountName && serverNumber) {
      loadStats()
      loadSchedule('construction')
      loadConfig()
      loadCurrentForm()
    }
  }, [sessionValid, accountName, serverNumber])

  useEffect(() => {
    if (activeTab === 'current-form' && currentForm && !submissions && !loadingSubmissions) {
      loadSubmissions()
    }
  }, [activeTab, currentForm, submissions, loadingSubmissions, loadSubmissions])

  const loadOldForms = useCallback(async () => {
    if (!accountName || !serverNumber) return
    setLoadingOldForms(true)
    const { ok, data } = await api.listOldForms(accountName, serverNumber)
    setLoadingOldForms(false)
    if (ok && (data as { old_forms?: unknown[] })?.old_forms) {
      setOldForms((data as { old_forms: Array<{ archive_name: string; code: string; name: string; created_at: string; delete_date?: string }> }).old_forms)
    } else {
      setOldForms([])
    }
  }, [accountName, serverNumber])

  useEffect(() => {
    if (activeTab === 'current-form') {
      loadOldForms()
    }
  }, [activeTab, loadOldForms])

  async function handleReopenForm(archiveName: string) {
    if (!accountName || !serverNumber) return
    setReopeningForm(true)
    const { ok, error } = await api.reopenForm(accountName, serverNumber, archiveName)
    setReopeningForm(false)
    if (ok) {
      loadCurrentForm()
      loadOldForms()
    } else {
      setConfigStatus({ type: 'error', message: error ?? 'Failed to reopen form' })
    }
  }

  async function handleClearSchedule(day?: 'construction' | 'research' | 'troops') {
    if (!accountName || !serverNumber) return
    const key = day ?? 'all'
    setClearScheduleLoading(key)
    const { ok, error } = await api.clearSchedule(accountName, serverNumber, day)
    setClearScheduleLoading(null)
    if (ok) {
      loadSchedule(currentScheduleDay)
    } else {
      setScheduleError(error ?? 'Failed to clear schedule')
    }
  }

  function selectScheduleDay(day: ScheduleDayKey) {
    setCurrentScheduleDay(day)
    loadSchedule(day)
  }

  function startEditSlot(slot: { time: string; player?: string }) {
    setEditingSlot({ time: slot.time, player: slot.player ?? '' })
    setTimeout(() => slotInputRef.current?.focus(), 0)
  }

  function cancelEdit() {
    setEditingSlot(null)
  }

  async function saveSlot(slot: { time: string }) {
    if (!accountName || !serverNumber || savingSlot || !editingSlot) return
    setSavingSlot(true)
    const playerValue = editingSlot.player?.trim() ?? ''
    const { ok, error } = await api.updateScheduleSlot(
      accountName,
      serverNumber,
      currentScheduleDay,
      slot.time,
      playerValue
    )
    setSavingSlot(false)
    if (ok) {
      setEditingSlot(null)
      loadSchedule(currentScheduleDay)
    } else {
      alert('Error: ' + (error ?? 'Failed to update slot'))
    }
  }

  function getTotal(data: {
    construction_requests: number
    research_requests: number
    troops_requests: number
  }) {
    return data.construction_requests + data.research_requests + data.troops_requests
  }

  const sortedAlliances = stats?.alliance_counts
    ? Object.entries(stats.alliance_counts).sort((a, b) => getTotal(b[1]) - getTotal(a[1]))
    : []
  const sortedConstructionTimeSlots = stats?.construction_time_slot_popularity
    ? sortTimeSlots(stats.construction_time_slot_popularity, stats.construction_start_time ?? '00:00')
    : {}
  const sortedResearchTimeSlots = stats?.research_time_slot_popularity
    ? sortTimeSlots(stats.research_time_slot_popularity, stats.research_start_time ?? '00:00')
    : {}
  const sortedTroopsTimeSlots = stats?.troops_time_slot_popularity
    ? sortTimeSlots(stats.troops_time_slot_popularity, stats.troops_start_time ?? '00:00')
    : {}
  const sortedTimeSlots = stats?.time_slot_popularity
    ? Object.entries(stats.time_slot_popularity).sort((a, b) => a[0].localeCompare(b[0]))
    : []

  async function handleCreateForm(e?: React.FormEvent) {
    e?.preventDefault()
    if (!accountName || !serverNumber) return
    const alliances = config.alliances.filter((a) => a.trim() !== '')
    if (alliances.length === 0) {
      setConfigStatus({ type: 'error', message: 'Error: At least one alliance must be specified' })
      return
    }
    if (!config.kingdom_id?.trim()) {
      setConfigStatus({ type: 'error', message: 'Error: Kingdom ID is required' })
      return
    }
    setCreatingForm(true)
    setConfigStatus(null)
    setCreatedFormUrl(null)
    setCreatedFormCode(null)
    const request: CreateFormRequest = {
      name: config.form_name?.trim() || undefined,
      kingdom_id: config.kingdom_id.trim(),
      alliances,
      include_non_of_above: config.include_non_of_above,
      construction_truegold_mode: config.construction_truegold_mode,
      construction_times: { start_time: '00:00', end_time: undefined },
      research_times: { start_time: '00:00', end_time: undefined },
      troops_times: { start_time: '00:00', end_time: undefined },
      intro_text: STANDARD_INTRO_TEXT,
      support_person_name: config.support_person_name?.trim() || undefined,
    }
    const { ok, data, error } = await api.createForm(accountName, serverNumber, request)
    setCreatingForm(false)
    if (ok && data) {
      const url = (data as { url?: string }).url ?? (data as { form_url?: string }).form_url
      const code = (data as { code?: string }).code ?? (data as { form_code?: string }).form_code
      setCreatedFormUrl(url ?? null)
      setCreatedFormCode(code ?? null)
      setConfigStatus({ type: 'success', message: 'Form created successfully!' })
      await loadCurrentForm()
      selectTab('current-form')
    } else {
      setConfigStatus({ type: 'error', message: 'Error: ' + (error ?? 'Failed to create form') })
    }
  }

  async function handleDownloadCSV() {
    if (!accountName || !serverNumber) return
    setDownloadingCSV(true)
    try {
      const res = await api.downloadFormCsv(accountName, serverNumber)
      if (res.ok) {
        const blob = await res.blob()
        const url = URL.createObjectURL(blob)
        const a = document.createElement('a')
        a.href = url
        a.download = `form_submissions_${accountName}_${serverNumber}.csv`
        a.click()
        URL.revokeObjectURL(url)
      }
    } finally {
      setDownloadingCSV(false)
    }
  }

  async function handleUpload(e: React.FormEvent) {
    e.preventDefault()
    if (!accountName || !serverNumber || !selectedFile) {
      setUploadStatus({ type: 'error', message: 'Please select a file' })
      return
    }
    setUploading(true)
    setUploadStatus(null)
    const res = await api.uploadCsv(accountName, serverNumber, selectedFile)
    setUploading(false)
    if (res.ok && (res.data as { success?: boolean })?.success) {
      setUploadStatus({
        type: 'success',
        message: (res.data as { message?: string })?.message ?? 'Schedule generated successfully!',
      })
      setSelectedFile(null)
      loadSchedule(currentScheduleDay)
      loadStats()
    } else {
      setUploadStatus({ type: 'error', message: 'Error: ' + (res.error ?? 'Upload failed') })
    }
  }

  async function lookupPlayer(index: number) {
    const slot = predeterminedSlots[index]
    if (!slot.player_id?.trim()) {
      setPredeterminedSlots((prev) => {
        const next = [...prev]
        next[index] = { ...next[index], alliance: '', name: '', lookupError: null }
        return next
      })
      return
    }
    setPredeterminedSlots((prev) => {
      const next = [...prev]
      next[index] = { ...next[index], lookingUp: true, lookupError: null }
      return next
    })
    const { ok, data } = await api.getPlayerById(accountName!, serverNumber!, slot.player_id!.trim())
    const player = (data as { player?: { name?: string; alliance?: string } })?.player
    setPredeterminedSlots((prev) => {
      const next = [...prev]
      next[index] = {
        ...next[index],
        lookingUp: false,
        alliance: player?.alliance ?? '',
        name: player?.name ?? '',
        lookupError: ok && player ? null : (data as { error?: string })?.error ?? 'Player not found',
      }
      return next
    })
  }

  async function savePredeterminedSlots(): Promise<boolean> {
    if (!currentForm || !accountName || !serverNumber) return false
    const slots = predeterminedSlots.map((s) => ({
      day: s.day,
      time: s.time,
      player_id: s.player_id || undefined,
      alliance: s.alliance,
      name: s.name,
    }))
    const { ok } = await api.updateFormConfig(accountName, serverNumber, slots)
    if (ok) await loadCurrentForm()
    return !!ok
  }

  async function handleGenerateSchedule(append: boolean, day?: 'construction' | 'research' | 'troops') {
    if (!accountName || !serverNumber) return
    setGeneratingSchedule(true)
    setScheduleGenStatus(null)
    if (predeterminedSlots.length > 0 && !day) {
      const saved = await savePredeterminedSlots()
      if (!saved) {
        setScheduleGenStatus({
          type: 'error',
          message: 'Failed to save predetermined slots. Please try again.',
        })
        setGeneratingSchedule(false)
        return
      }
    }
    const { ok, data, error } = await api.generateSchedule(append, day)
    setGeneratingSchedule(false)
    if (ok && (data as { success?: boolean })?.success) {
      setScheduleGenStatus({
        type: 'success',
        message: (data as { message?: string })?.message ?? 'Schedule generated successfully!',
      })
      loadSchedule(currentScheduleDay)
      loadStats()
    } else {
      setScheduleGenStatus({
        type: 'error',
        message: (data as { error?: string })?.error ?? error ?? 'Failed to generate schedule',
      })
    }
  }

  function copyToClipboard(text: string, inputId?: string) {
    const fullUrl = text.startsWith('/') ? `${window.location.origin}${text}` : text
    if (navigator.clipboard?.writeText) {
      navigator.clipboard.writeText(fullUrl)
    } else if (inputId) {
      const input = document.getElementById(inputId) as HTMLInputElement
      if (input) {
        input.value = fullUrl
        input.select()
        document.execCommand('copy')
      }
    }
  }

  function formatDate(dateString?: string) {
    if (!dateString) return 'Unknown'
    try {
      return new Date(dateString).toLocaleString()
    } catch {
      return dateString
    }
  }

  if (sessionValid === null) {
    return (
      <div className="container mx-auto px-4 py-8 flex justify-center items-center min-h-[50vh]">
        <i className="fas fa-spinner fa-spin text-4xl text-blue-400"></i>
      </div>
    )
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-7xl">
      <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
        <div className="min-h-[200px]">
          {/* Profile Tab */}
          {activeTab === 'profile' && (
            <div className="max-w-2xl mx-auto">
              {/* Profile header */}
              <div className="flex items-start gap-6 mb-8">
                <div className="w-20 h-20 rounded-full bg-purple-600 flex items-center justify-center text-white font-semibold text-2xl flex-shrink-0 overflow-hidden relative">
                  <span className="absolute inset-0 flex items-center justify-center">
                    {(inGameName || accountName) ? (inGameName || accountName)!.charAt(0).toUpperCase() : '?'}
                  </span>
                  {playerId && (
                    <img
                      key={playerId}
                      src={`/api/avatar/${playerId}`}
                      alt=""
                      className="w-full h-full object-cover absolute inset-0 z-10"
                      onError={(e) => {
                        (e.target as HTMLImageElement).style.display = 'none'
                      }}
                    />
                  )}
                </div>
                <div className="flex-1 min-w-0">
                  <h2 className="text-2xl font-bold text-white mb-1">
                    {inGameName || (accountName ? accountName.charAt(0).toUpperCase() + accountName.slice(1) : 'Account')}
                  </h2>
                  <p className="text-gray-400 text-sm mb-4">Schedule Maker account</p>
                </div>
              </div>

              {/* Editable profile fields */}
              <div className="space-y-4 mb-8">
                <div>
                  <label className="block text-xs font-medium text-gray-500 uppercase tracking-wider mb-2">Account name</label>
                  <input
                    type="text"
                    value={profileEdit.account_name}
                    onChange={(e) => setProfileEdit((p) => ({ ...p, account_name: e.target.value }))}
                    className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                    placeholder="Account name (used in URL)"
                  />
                </div>
                <div>
                  <label className="block text-xs font-medium text-gray-500 uppercase tracking-wider mb-2">Server</label>
                  <input
                    type="number"
                    value={profileEdit.server_number}
                    onChange={(e) => setProfileEdit((p) => ({ ...p, server_number: e.target.value }))}
                    min={1}
                    className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                    placeholder="Server number"
                  />
                </div>
                <div>
                  <label className="block text-xs font-medium text-gray-500 uppercase tracking-wider mb-2">In-game name</label>
                  <input
                    type="text"
                    value={profileEdit.in_game_name}
                    onChange={(e) => setProfileEdit((p) => ({ ...p, in_game_name: e.target.value }))}
                    className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                    placeholder="Your Kingshot character name"
                  />
                </div>
                <button
                  onClick={async () => {
                    setProfileSaving(true)
                    setProfileError(null)
                    const { ok, data, error } = await api.updateProfile({
                      account_name: profileEdit.account_name.trim() || undefined,
                      server_number: profileEdit.server_number ? parseInt(profileEdit.server_number, 10) : undefined,
                      in_game_name: profileEdit.in_game_name.trim() || undefined,
                    })
                    setProfileSaving(false)
                    if (ok && data?.success) {
                      setServerNumber(data.server_number ?? serverNumber)
                      setInGameName(data.in_game_name ?? null)
                      setProfileEdit((p) => ({
                        ...p,
                        account_name: data.account_name ?? p.account_name,
                        server_number: String(data.server_number ?? p.server_number),
                        in_game_name: data.in_game_name ?? p.in_game_name,
                      }))
                      await refreshAuth()
                      if (data.account_name && data.account_name !== accountName) {
                        navigate(`/dashboard/${data.account_name}?tab=profile`, { replace: true })
                      }
                    } else {
                      setProfileError((data as { error?: string })?.error || error || 'Failed to update')
                    }
                  }}
                  disabled={profileSaving}
                  className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded-lg font-medium"
                >
                  {profileSaving ? <i className="fas fa-spinner fa-spin mr-2"></i> : null}
                  Save changes
                </button>
                {profileError && <p className="text-red-400 text-sm">{profileError}</p>}
              </div>

              {/* Kingshot ID lookup */}
              <div className="bg-gray-700/50 rounded-xl p-6 border border-gray-600 mb-8">
                <h3 className="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">Kingshot ID</h3>
                <p className="text-gray-400 text-sm mb-3">
                  Enter your Kingshot player ID to auto-update name, server, and profile picture from the game.
                </p>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={kingshotIdInput}
                    onChange={(e) => setKingshotIdInput(e.target.value)}
                    placeholder="Enter Kingshot player ID"
                    className="flex-1 px-4 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                  />
                  <button
                    onClick={async () => {
                      if (!kingshotIdInput.trim()) return
                      setKingshotLookingUp(true)
                      setKingshotError(null)
                      const { ok, data, error } = await api.kingshotLookup(kingshotIdInput.trim())
                      setKingshotLookingUp(false)
                      if (ok && data?.success) {
                        setInGameName(data.in_game_name ?? null)
                        setServerNumber(data.server_number ?? null)
                        setPlayerId(data.player_id ?? null)
                        setProfileEdit((p) => ({
                          ...p,
                          in_game_name: data.in_game_name ?? p.in_game_name,
                          server_number: String(data.server_number ?? p.server_number),
                        }))
                        await refreshAuth()
                      } else {
                        setKingshotError((data as { error?: string })?.error || error || 'Lookup failed')
                      }
                    }}
                    disabled={kingshotLookingUp}
                    className="px-4 py-2 bg-green-600 hover:bg-green-700 disabled:opacity-50 text-white rounded-lg font-medium"
                  >
                    {kingshotLookingUp ? <i className="fas fa-spinner fa-spin mr-2"></i> : null}
                    Confirm
                  </button>
                </div>
                {kingshotError && <p className="text-red-400 text-sm mt-2">{kingshotError}</p>}
                {playerId && <p className="text-gray-500 text-sm mt-2">Current ID: {playerId}</p>}
              </div>

              {/* Quick actions / Linked schedule */}
              <div className="bg-gray-700/50 rounded-xl p-6 border border-gray-600">
                <div className="flex items-center justify-between mb-4">
                  <h3 className="text-sm font-medium text-gray-400 uppercase tracking-wider">Your Schedule</h3>
                  {accountName && serverNumber != null && (
                    <span className="text-xs text-green-400 font-medium">Active</span>
                  )}
                </div>
                {accountName && serverNumber != null ? (
                  <div className="flex items-center justify-between gap-4 p-4 bg-gray-800/50 rounded-lg border border-gray-600">
                    <div>
                      <p className="font-semibold text-white">{accountName}</p>
                      <p className="text-sm text-gray-400">Server {serverNumber}</p>
                    </div>
                    <Link
                      to={`/${accountName}/${serverNumber}`}
                      className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm font-medium transition-all flex items-center gap-2"
                    >
                      <i className="fas fa-external-link-alt"></i>
                      Open
                    </Link>
                  </div>
                ) : (
                  <p className="text-gray-500 text-sm">No schedule linked</p>
                )}
              </div>
            </div>
          )}

          {/* Schedule Tab */}
          {activeTab === 'schedule' && (
            <div>
              <div className="text-center mb-8">
                <div className="inline-block bg-orange-900/50 rounded-full p-4 mb-4">
                  <i className="fas fa-calendar-check text-orange-400 text-3xl"></i>
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">Schedule</h2>
                <p className="text-gray-400">View appointment schedules for each day</p>
              </div>

              <div className="bg-gray-800 rounded-xl shadow-xl p-6 mb-6 border border-gray-700">
                <p className="text-xs font-medium text-gray-500 uppercase tracking-wider mb-3">View schedule</p>
                <div className="flex flex-col sm:flex-row gap-2 mb-4">
                  {(Object.keys(SCHEDULE_DAYS) as ScheduleDayKey[]).map((key) => (
                    <button
                      key={key}
                      onClick={() => selectScheduleDay(key)}
                      className={`flex items-center justify-center gap-2 px-5 py-3 rounded-lg font-semibold transition-all ${
                        SCHEDULE_DAYS[key].buttonClass
                      } ${currentScheduleDay === key ? SCHEDULE_DAYS[key].ringClass : 'opacity-90 hover:opacity-100'}`}
                    >
                      <i className={`${SCHEDULE_DAYS[key].icon}`}></i>
                      {SCHEDULE_DAYS[key].name}
                    </button>
                  ))}
                </div>
                <div className="flex flex-wrap gap-2">
                  <button
                    onClick={() => handleClearSchedule()}
                    disabled={!!clearScheduleLoading}
                    className="px-4 py-2 bg-red-600/90 hover:bg-red-600 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
                  >
                    {clearScheduleLoading === 'all' ? <i className="fas fa-spinner fa-spin mr-2"></i> : <i className="fas fa-trash mr-2"></i>}
                    Clear All
                  </button>
                  <button
                    onClick={() => handleClearSchedule(currentScheduleDay)}
                    disabled={!!clearScheduleLoading}
                    className="px-3 py-2 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-gray-300 rounded-lg text-sm font-medium transition-colors flex items-center gap-1.5"
                  >
                    {clearScheduleLoading === currentScheduleDay ? (
                      <i className="fas fa-spinner fa-spin"></i>
                    ) : (
                      <i className={`${SCHEDULE_DAYS[currentScheduleDay].icon} text-gray-400`}></i>
                    )}
                    Clear {SCHEDULE_DAYS[currentScheduleDay].name}
                  </button>
                </div>
              </div>

              {scheduleLoading && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-spinner fa-spin text-4xl text-blue-400 mb-4"></i>
                  <p className="text-xl text-gray-400">Loading schedule...</p>
                </div>
              )}

              {!scheduleLoading && scheduleError && (
                <div className="bg-red-900/50 border-l-4 border-red-500 text-red-200 p-4 rounded-lg">
                  <i className="fas fa-exclamation-circle mr-2"></i>
                  {scheduleError}
                </div>
              )}

              {!scheduleLoading && !scheduleError && currentSchedule && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                  <h2 className="text-3xl font-bold text-white mb-6 text-center">
                    {currentSchedule.day_name}
                  </h2>
                  <p className="text-sm text-gray-400 mb-4 text-center">
                    Click on any slot to edit the player name
                  </p>
                  <div className="border-2 border-gray-700 rounded-lg overflow-hidden">
                    {currentSchedule.appointments?.map((slot) => (
                      <div
                        key={slot.time}
                        onClick={() => startEditSlot(slot)}
                        className={`flex items-center p-3 border-b border-gray-700 hover:bg-gray-700/50 transition-colors cursor-pointer ${
                          slot.is_empty ? 'opacity-60' : ''
                        } ${editingSlot?.time === slot.time ? 'bg-blue-900/30' : ''}`}
                      >
                        <span
                          className={
                            slot.is_empty
                              ? 'w-24 font-bold text-gray-500'
                              : 'w-24 font-bold text-blue-400'
                          }
                        >
                          {slot.time}
                        </span>
                        {editingSlot?.time === slot.time ? (
                          <div className="flex-1 flex items-center gap-2">
                            <input
                              ref={slotInputRef}
                              type="text"
                              value={editingSlot.player}
                              onChange={(e) =>
                                setEditingSlot((p) => (p ? { ...p, player: e.target.value } : null))
                              }
                              onKeyDown={(e) => {
                                if (e.key === 'Enter') saveSlot(slot)
                                if (e.key === 'Escape') cancelEdit()
                              }}
                              className="flex-1 px-3 py-1 bg-gray-700 border border-blue-500 rounded text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                              placeholder="[alliance] name or empty to clear"
                              onClick={(e) => e.stopPropagation()}
                            />
                            <button
                              onClick={(e) => {
                                e.stopPropagation()
                                saveSlot(slot)
                              }}
                              className="px-3 py-1 bg-green-600 hover:bg-green-700 text-white rounded transition-colors"
                            >
                              <i className="fas fa-check"></i>
                            </button>
                            <button
                              onClick={(e) => {
                                e.stopPropagation()
                                cancelEdit()
                              }}
                              className="px-3 py-1 bg-red-600 hover:bg-red-700 text-white rounded transition-colors"
                            >
                              <i className="fas fa-times"></i>
                            </button>
                          </div>
                        ) : (
                          <div className="flex-1">
                            {slot.is_empty ? (
                              <span className="text-gray-500 italic">[EMPTY]</span>
                            ) : (
                              <span className="text-gray-200 font-medium">{slot.player}</span>
                            )}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {!scheduleLoading && !scheduleError && !currentSchedule && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-calendar-times text-4xl text-gray-500 mb-4"></i>
                  <p className="text-xl text-gray-400">
                    No schedule data available. Generate a schedule first.
                  </p>
                </div>
              )}
            </div>
          )}

          {/* Statistics Tab */}
          {activeTab === 'stats' && (
            <div>
              <div className="text-center mb-8">
                <div className="inline-block bg-blue-900/50 rounded-full p-4 mb-4">
                  <i className="fas fa-chart-bar text-blue-400 text-3xl"></i>
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">Statistics</h2>
                <p className="text-gray-400">View alliance and time slot statistics</p>
              </div>

              {statsLoading && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-spinner fa-spin text-4xl text-blue-400 mb-4"></i>
                  <p className="text-xl text-gray-400">Loading statistics...</p>
                </div>
              )}

              {!statsLoading && statsError && (
                <div className="bg-red-900/50 border-l-4 border-red-500 text-red-200 p-4 rounded-lg">
                  <i className="fas fa-exclamation-circle mr-2"></i>
                  {statsError}
                </div>
              )}

              {!statsLoading && !statsError && stats && (
                <div className="space-y-8">
                  <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                    <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                      <i className="fas fa-users text-blue-400 mr-3"></i>Alliance Request Counts
                    </h2>
                    <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-4">
                      {sortedAlliances.map(([alliance, allianceData]) => (
                        <div
                          key={alliance}
                          className="bg-gray-700/50 rounded-xl p-5 border-2 border-gray-600 hover:border-blue-500 hover:shadow-lg transition-all"
                        >
                          <h3 className="text-xl font-bold text-white mb-4">
                            {alliance || '(No Alliance)'}
                          </h3>
                          <div className="space-y-2">
                            <div className="flex justify-between items-center">
                              <span className="text-gray-300 flex items-center">
                                <i className="fas fa-hammer text-orange-400 mr-2"></i>Construction:
                              </span>
                              <strong className="text-orange-400 text-lg">
                                {allianceData.construction_requests}
                              </strong>
                            </div>
                            <div className="flex justify-between items-center">
                              <span className="text-gray-300 flex items-center">
                                <i className="fas fa-flask text-blue-400 mr-2"></i>Research:
                              </span>
                              <strong className="text-blue-400 text-lg">
                                {allianceData.research_requests}
                              </strong>
                            </div>
                            <div className="flex justify-between items-center">
                              <span className="text-gray-300 flex items-center">
                                <i className="fas fa-users text-green-400 mr-2"></i>Troops:
                              </span>
                              <strong className="text-green-400 text-lg">
                                {allianceData.troops_requests}
                              </strong>
                            </div>
                            <div className="flex justify-between items-center pt-3 border-t-2 border-gray-600 mt-3">
                              <span className="font-bold text-gray-200">Total:</span>
                              <strong className="text-blue-400 text-xl">{getTotal(allianceData)}</strong>
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>

                  {Object.keys(sortedConstructionTimeSlots).length > 0 && (
                    <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                      <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                        <i className="fas fa-hammer text-orange-400 mr-3"></i>Construction Day Time
                        Slot Popularity
                      </h2>
                      <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-7 gap-2">
                        {Object.entries(sortedConstructionTimeSlots).map(([time, timeData]) => (
                          <div
                            key={'construction-' + time}
                            className="bg-gray-700/50 rounded-lg p-2 border border-gray-600 hover:border-orange-500 hover:shadow-md transition-all text-center"
                          >
                            <div className="font-bold text-orange-400 text-sm mb-1">{time}</div>
                            <div className="text-xl font-bold text-white">{timeData.requests}</div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {Object.keys(sortedResearchTimeSlots).length > 0 && (
                    <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                      <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                        <i className="fas fa-flask text-blue-400 mr-3"></i>Research Day Time Slot
                        Popularity
                      </h2>
                      <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-7 gap-2">
                        {Object.entries(sortedResearchTimeSlots).map(([time, timeData]) => (
                          <div
                            key={'research-' + time}
                            className="bg-gray-700/50 rounded-lg p-2 border border-gray-600 hover:border-blue-500 hover:shadow-md transition-all text-center"
                          >
                            <div className="font-bold text-blue-400 text-sm mb-1">{time}</div>
                            <div className="text-xl font-bold text-white">{timeData.requests}</div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {Object.keys(sortedTroopsTimeSlots).length > 0 && (
                    <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                      <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                        <i className="fas fa-users text-green-400 mr-3"></i>Troops Training Day Time
                        Slot Popularity
                      </h2>
                      <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-7 gap-2">
                        {Object.entries(sortedTroopsTimeSlots).map(([time, timeData]) => (
                          <div
                            key={'troops-' + time}
                            className="bg-gray-700/50 rounded-lg p-2 border border-gray-600 hover:border-green-500 hover:shadow-md transition-all text-center"
                          >
                            <div className="font-bold text-green-400 text-sm mb-1">{time}</div>
                            <div className="text-xl font-bold text-white">{timeData.requests}</div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {Object.keys(sortedConstructionTimeSlots).length === 0 &&
                    sortedTimeSlots.length > 0 && (
                      <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                        <h2 className="text-3xl font-bold text-white mb-6 flex items-center">
                          <i className="fas fa-clock text-blue-400 mr-3"></i>Time Slot Popularity
                        </h2>
                        <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-7 gap-2">
                          {sortedTimeSlots.map(([time, timeData]) => (
                            <div
                              key={time}
                              className="bg-gray-700/50 rounded-lg p-2 border border-gray-600 hover:border-blue-500 hover:shadow-md transition-all text-center"
                            >
                              <div className="font-bold text-blue-400 text-sm mb-1">{time}</div>
                              <div className="space-y-1 text-xs">
                                <div className="flex justify-between">
                                  <span className="text-orange-400">
                                    <i className="fas fa-hammer"></i>
                                  </span>
                                  <strong className="text-gray-200">
                                    {timeData.construction_requests}
                                  </strong>
                                </div>
                                <div className="flex justify-between">
                                  <span className="text-blue-400">
                                    <i className="fas fa-flask"></i>
                                  </span>
                                  <strong className="text-gray-200">
                                    {timeData.research_requests}
                                  </strong>
                                </div>
                                <div className="flex justify-between">
                                  <span className="text-green-400">
                                    <i className="fas fa-users"></i>
                                  </span>
                                  <strong className="text-gray-200">
                                    {timeData.troops_requests}
                                  </strong>
                                </div>
                              </div>
                              <div className="mt-1 pt-1 border-t border-gray-600 font-bold text-blue-400 text-xs">
                                {getTotal(timeData)}
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                </div>
              )}

              {!statsLoading && !statsError && !stats && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-chart-bar text-4xl text-gray-500 mb-4"></i>
                  <p className="text-xl text-gray-400">
                    No statistics available. Generate a schedule first.
                  </p>
                </div>
              )}
            </div>
          )}

          {/* Create Form Tab */}
          {activeTab === 'create-form' && (
            <div>
              <div className="text-center mb-8">
                <div className="inline-block bg-purple-900/50 rounded-full p-4 mb-4">
                  <i className="fas fa-cog text-purple-400 text-3xl"></i>
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">Create Form</h2>
                <p className="text-gray-400">
                  Configure alliances and schedule times, then create a form to get a shareable link
                </p>
              </div>

              {createdFormUrl && (
                <div className="mb-8 p-6 bg-green-900/50 border-l-4 border-green-500 rounded-lg">
                  <h3 className="text-xl font-bold text-green-200 mb-3">
                    <i className="fas fa-check-circle mr-2"></i>Form Created Successfully!
                  </h3>
                  <p className="text-gray-300 mb-3">Share this link with your players to fill out the form:</p>
                  <div className="flex items-center gap-2 bg-gray-800 p-3 rounded-lg border border-gray-700 mb-3">
                    <input
                      id="formUrlInput"
                      type="text"
                      value={createdFormUrl}
                      readOnly
                      className="flex-1 bg-transparent text-white font-mono text-sm outline-none"
                    />
                    <button
                      onClick={() => copyToClipboard(createdFormUrl, 'formUrlInput')}
                      className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-all"
                    >
                      <i className="fas fa-copy mr-2"></i>Copy
                    </button>
                  </div>
                  <div className="flex gap-2 flex-wrap">
                    <a
                      href={createdFormUrl}
                      target="_blank"
                      rel="noreferrer"
                      className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg transition-all text-sm"
                    >
                      <i className="fas fa-external-link-alt mr-2"></i>Open Form
                    </a>
                    <a
                      href={createdFormUrl + '/stats'}
                      target="_blank"
                      rel="noreferrer"
                      className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-all text-sm"
                    >
                      <i className="fas fa-chart-bar mr-2"></i>View Statistics
                    </a>
                  </div>
                  {createdFormCode && (
                    <p className="text-sm text-gray-400 mt-3">
                      Form Code: <span className="font-mono text-green-300">{createdFormCode}</span>
                    </p>
                  )}
                </div>
              )}

              <div className="space-y-8">
                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-white mb-4">
                    <i className="fas fa-tag mr-2"></i>Form Name
                  </h3>
                  <p className="text-sm text-gray-400 mb-4">
                    Give this form a name (e.g., &quot;Week 1&quot;, &quot;January 2025&quot;). Optional.
                  </p>
                  <input
                    value={config.form_name}
                    onChange={(e) => setConfig((c) => ({ ...c, form_name: e.target.value }))}
                    type="text"
                    className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                    placeholder="Enter form name (optional)"
                  />
                </div>

                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-white mb-4">
                    <i className="fas fa-crown mr-2"></i>Kingdom ID <span className="text-red-400">*</span>
                  </h3>
                  <p className="text-sm text-gray-400 mb-4">
                    The kingdom ID used to validate applicants.
                  </p>
                  <input
                    value={config.kingdom_id}
                    onChange={(e) => setConfig((c) => ({ ...c, kingdom_id: e.target.value }))}
                    type="text"
                    required
                    className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                    placeholder="e.g. 123 or 456"
                  />
                </div>

                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-white mb-4">
                    <i className="fas fa-info-circle mr-2"></i>Introduction Text
                  </h3>
                  <p className="text-sm text-gray-400 mb-4">Standard introduction text (fixed).</p>
                  <textarea
                    value={STANDARD_INTRO_TEXT}
                    readOnly
                    disabled
                    rows={15}
                    className="w-full px-4 py-2 bg-gray-800 border border-gray-600 rounded-lg text-gray-400 cursor-not-allowed font-mono text-sm"
                  />
                </div>

                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-white mb-4">
                    <i className="fas fa-user-headset mr-2"></i>Support Person&apos;s Name
                  </h3>
                  <input
                    value={config.support_person_name}
                    onChange={(e) => setConfig((c) => ({ ...c, support_person_name: e.target.value }))}
                    type="text"
                    className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                    placeholder="e.g. [COB]Vor"
                  />
                </div>

                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-white mb-4">
                    <i className="fas fa-users mr-2"></i>Alliances <span className="text-red-400">*</span>
                  </h3>
                  <p className="text-sm text-gray-400 mb-4">Add or remove alliance names.</p>
                  <button
                    type="button"
                    onClick={() =>
                      setConfig((c) => ({
                        ...c,
                        include_non_of_above: !c.include_non_of_above,
                      }))
                    }
                    className={`w-full px-4 py-3 rounded-lg font-medium transition-all flex items-center justify-center gap-2 mb-4 ${
                      config.include_non_of_above
                        ? 'bg-green-600/30 border-2 border-green-500 text-green-200 hover:bg-green-600/50'
                        : 'bg-gray-700 border-2 border-gray-600 text-gray-400 hover:bg-gray-600'
                    }`}
                  >
                    <i
                      className={
                        config.include_non_of_above ? 'fas fa-check-circle' : 'fas fa-times-circle'
                      }
                    ></i>
                    {config.include_non_of_above
                      ? 'Include "Non of the above"'
                      : 'Exclude "Non of the above"'}
                  </button>
                  <div className="space-y-3">
                    {config.alliances.map((alliance, index) => (
                      <div key={index} className="flex items-center gap-2">
                        <input
                          value={alliance}
                          onChange={(e) =>
                            setConfig((c) => {
                              const next = [...c.alliances]
                              next[index] = e.target.value
                              return { ...c, alliances: next }
                            })
                          }
                          type="text"
                          className="flex-1 px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                          placeholder="Alliance name"
                        />
                        <button
                          onClick={() =>
                            setConfig((c) => ({
                              ...c,
                              alliances: c.alliances.filter((_, i) => i !== index),
                            }))
                          }
                          className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg transition-all"
                        >
                          <i className="fas fa-trash"></i>
                        </button>
                      </div>
                    ))}
                    <button
                      onClick={() =>
                        setConfig((c) => ({ ...c, alliances: [...c.alliances, ''] }))
                      }
                      className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-all"
                    >
                      <i className="fas fa-plus mr-2"></i>Add Alliance
                    </button>
                  </div>
                </div>

                <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600">
                  <h3 className="text-xl font-bold text-amber-400 mb-4">
                    <i className="fas fa-coins mr-2"></i>Current Age of Your Kingdom <span className="text-red-400">*</span>
                  </h3>
                  <div className="space-y-2">
                    {[
                      {
                        value: 'pre_truegold',
                        label: 'Pre-truegold',
                        desc: 'Neither truegold, tempered truegold, nor truegold dust unlocked',
                      },
                      {
                        value: 'truegold_unlocked',
                        label: 'Truegold unlocked',
                        desc: 'Truegold unlocked but not tempered truegold',
                      },
                      {
                        value: 'war_academy_unlocked',
                        label: 'War academy unlocked',
                        desc: 'Truegold and truegold dust unlocked, but not tempered truegold',
                      },
                      {
                        value: 'tempered_truegold_unlocked',
                        label: 'Tempered truegold unlocked',
                        desc: 'Truegold, tempered truegold, and truegold dust unlocked',
                      },
                    ].map((opt) => (
                      <label
                        key={opt.value}
                        className={`flex items-start gap-3 p-3 rounded-lg cursor-pointer hover:bg-gray-600/50 transition-colors ${
                          config.construction_truegold_mode === opt.value
                            ? 'bg-amber-900/30 border border-amber-600/50'
                            : 'border border-transparent'
                        }`}
                      >
                        <input
                          type="radio"
                          checked={config.construction_truegold_mode === opt.value}
                          onChange={() =>
                            setConfig((c) => ({
                              ...c,
                              construction_truegold_mode: opt.value,
                            }))
                          }
                          className="mt-1"
                        />
                        <div>
                          <span className="font-semibold text-white">{opt.label}</span>
                          <p className="text-sm text-gray-400">{opt.desc}</p>
                        </div>
                      </label>
                    ))}
                  </div>
                </div>

                <button
                  onClick={() => handleCreateForm()}
                  disabled={creatingForm}
                  className="w-full bg-purple-600 hover:bg-purple-700 text-white px-6 py-3 rounded-lg font-semibold transition-all shadow-lg hover:shadow-xl disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <i className={`fas ${creatingForm ? 'fa-spinner fa-spin' : 'fa-plus-circle'} mr-2`}></i>
                  {creatingForm ? 'Creating Form...' : 'Create Form'}
                </button>

                {configStatus && (
                  <div
                    className={`p-4 rounded-lg ${
                      configStatus.type === 'success'
                        ? 'bg-green-900/50 border-l-4 border-green-500 text-green-200'
                        : 'bg-red-900/50 border-l-4 border-red-500 text-red-200'
                    }`}
                  >
                    <i
                      className={`fas ${
                        configStatus.type === 'success' ? 'fa-check-circle' : 'fa-times-circle'
                      } mr-2`}
                    ></i>
                    {configStatus.message}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Current Form Tab */}
          {activeTab === 'current-form' && (
            <div>
              <div className="text-center mb-8">
                <div className="inline-block bg-purple-900/50 rounded-full p-4 mb-4">
                  <i className="fas fa-file-alt text-purple-400 text-3xl"></i>
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">Current Form</h2>
                <p className="text-gray-400">View and manage your current form</p>
              </div>

              {loadingCurrentForm && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-spinner fa-spin text-4xl text-blue-400 mb-4"></i>
                  <p className="text-xl text-gray-400">Loading form information...</p>
                </div>
              )}

              {!loadingCurrentForm && currentForm && (
                <div className="space-y-6">
                  <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                    <div className="space-y-4">
                      <div>
                        <p className="text-sm text-gray-300 mb-1">Form Name:</p>
                        <p className="text-2xl font-bold text-white">{currentForm.name}</p>
                      </div>
                      <div>
                        <p className="text-sm text-gray-300 mb-1">Form Code:</p>
                        <p className="text-lg font-mono text-purple-300">{currentForm.code}</p>
                      </div>
                      <div>
                        <p className="text-sm text-gray-300 mb-1">Created:</p>
                        <p className="text-gray-200">{formatDate(currentForm.created_at)}</p>
                      </div>
                      {(currentForm as { delete_date?: string }).delete_date && (
                        <div>
                          <p className="text-sm text-gray-300 mb-1">Auto-archives:</p>
                          <p className="text-amber-400">{(currentForm as { delete_date?: string }).delete_date}</p>
                        </div>
                      )}
                      <div>
                        <p className="text-sm text-gray-300 mb-1">Responses:</p>
                        <p className="text-3xl font-bold text-green-400">
                          {currentForm.submissions_count ?? 0}
                        </p>
                      </div>
                    </div>
                  </div>

                  <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                    <h3 className="text-xl font-bold text-white mb-4">Form Link</h3>
                    <div className="flex items-center gap-2 bg-gray-700 p-3 rounded-lg border border-gray-600 mb-4">
                      <input
                        id="currentFormUrlInput"
                        type="text"
                        value={currentForm.url}
                        readOnly
                        className="flex-1 bg-transparent text-white font-mono text-sm outline-none"
                      />
                      <button
                        onClick={() => copyToClipboard(currentForm.url, 'currentFormUrlInput')}
                        className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg transition-all"
                      >
                        <i className="fas fa-copy mr-2"></i>Copy Link
                      </button>
                    </div>
                    <div className="flex gap-2 flex-wrap">
                      <a
                        href={currentForm.url}
                        target="_blank"
                        rel="noreferrer"
                        className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg transition-all"
                      >
                        <i className="fas fa-external-link-alt mr-2"></i>Open Form
                      </a>
                      <a
                        href={currentForm.url + '/stats'}
                        target="_blank"
                        rel="noreferrer"
                        className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-all"
                      >
                        <i className="fas fa-chart-bar mr-2"></i>View Statistics
                      </a>
                    </div>
                  </div>

                  <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                    <div className="flex justify-between items-center mb-4">
                      <h3 className="text-xl font-bold text-white">
                        <i className="fas fa-table mr-2"></i>All Form Submissions
                      </h3>
                      <button
                        onClick={loadSubmissions}
                        disabled={loadingSubmissions}
                        className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-semibold transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        <i
                          className={`fas ${
                            loadingSubmissions ? 'fa-spinner fa-spin' : 'fa-sync-alt'
                          } mr-2`}
                        ></i>
                        {loadingSubmissions ? 'Reloading...' : 'Reload'}
                      </button>
                    </div>
                    {loadingSubmissions && (
                      <div className="text-center py-8">
                        <i className="fas fa-spinner fa-spin text-2xl text-blue-400 mb-2"></i>
                        <p className="text-gray-400">Loading submissions...</p>
                      </div>
                    )}
                    {!loadingSubmissions && submissionsError && (
                      <div className="bg-red-900/50 border-l-4 border-red-500 text-red-200 p-4 rounded-lg">
                        <i className="fas fa-exclamation-circle mr-2"></i>
                        {submissionsError}
                      </div>
                    )}
                    {!loadingSubmissions && !submissionsError && submissions && submissions.length > 0 && (
                      <div className="overflow-x-auto">
                        <table className="min-w-full text-left border-collapse" style={{ minWidth: 1600 }}>
                          <thead>
                            <tr className="border-b border-gray-700">
                              {SUBMISSION_HEADERS.map((header) => (
                                <th
                                  key={header}
                                  className={`px-4 py-3 text-gray-300 font-semibold bg-gray-700/50 whitespace-nowrap ${
                                    ['Troop times', 'Construction times', 'Research times'].includes(header)
                                      ? 'min-w-[300px]'
                                      : ''
                                  }`}
                                >
                                  {header}
                                </th>
                              ))}
                            </tr>
                          </thead>
                          <tbody>
                            {submissions.map((submission, index) => (
                              <tr
                                key={index}
                                className="border-b border-gray-700 hover:bg-gray-700/30 transition-colors"
                              >
                                {SUBMISSION_HEADERS.map((header) => (
                                  <td
                                    key={header}
                                    className={`px-4 py-3 text-gray-200 ${
                                      ['Troop times', 'Construction times', 'Research times'].includes(
                                        header
                                      )
                                        ? 'min-w-[300px]'
                                        : ''
                                    }`}
                                  >
                                    {getSubmissionValue(
                                      submission as Record<string, unknown>,
                                      header
                                    )}
                                  </td>
                                ))}
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                    )}
                    {!loadingSubmissions && !submissionsError && (!submissions || submissions.length === 0) && (
                      <div className="text-center py-8 text-gray-400">
                        <i className="fas fa-inbox text-4xl mb-2"></i>
                        <p>No submissions yet</p>
                      </div>
                    )}
                  </div>
                </div>
              )}

              {!loadingCurrentForm && !currentForm && (
                <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
                  <i className="fas fa-file-alt text-4xl text-gray-500 mb-4"></i>
                  <p className="text-xl text-gray-400 mb-4">No current form found.</p>
                  <p className="text-gray-500">Create a new form using the &quot;Create Form&quot; tab, or reopen an archived form below.</p>
                </div>
              )}

              <div className="bg-gray-800 rounded-lg shadow-xl p-8 mb-6 border border-gray-700 mt-8">
                <h3 className="text-xl font-bold text-white mb-4">
                  <i className="fas fa-history mr-2"></i>Reopen Archived Form
                </h3>
                <p className="text-gray-400 mb-4">Forms are archived 2 weeks after creation. Reopen a previously archived form to use it again.</p>
                {loadingOldForms ? (
                  <div className="text-center py-4">
                    <i className="fas fa-spinner fa-spin text-2xl text-blue-400"></i>
                  </div>
                ) : oldForms.length === 0 ? (
                  <p className="text-gray-500">No archived forms.</p>
                ) : (
                  <div className="space-y-2">
                    {oldForms.map((f) => (
                      <div
                        key={f.archive_name}
                        className="flex items-center justify-between gap-4 p-4 bg-gray-700/50 rounded-lg border border-gray-600"
                      >
                        <div>
                          <p className="font-semibold text-white">{f.name}</p>
                          <p className="text-sm text-gray-400">
                            Code: {f.code} • Created: {formatDate(f.created_at)}
                            {f.delete_date && ` • Archived: ${f.delete_date}`}
                          </p>
                        </div>
                        <button
                          onClick={() => handleReopenForm(f.archive_name)}
                          disabled={reopeningForm}
                          className="px-4 py-2 bg-purple-600 hover:bg-purple-700 disabled:opacity-50 text-white rounded-lg font-medium transition-all"
                        >
                          {reopeningForm ? <i className="fas fa-spinner fa-spin mr-2"></i> : <i className="fas fa-folder-open mr-2"></i>}
                          Reopen
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* CSV Operations Tab */}
          {activeTab === 'csv-operations' && (
            <div>
              <div className="text-center mb-8">
                <div className="inline-block bg-green-900/50 rounded-full p-4 mb-4">
                  <i className="fas fa-file-csv text-green-400 text-3xl"></i>
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">CSV Operations</h2>
                <p className="text-gray-400">Upload or download CSV files</p>
              </div>

              <div className="bg-gray-800 rounded-lg shadow-xl p-6 mb-6 border border-gray-700">
                <h3 className="text-xl font-bold text-white mb-4">
                  <i className="fas fa-download mr-2"></i>Download Current Form CSV
                </h3>
                <p className="text-gray-400 mb-4">
                  Download the CSV file containing all form submissions
                </p>
                <button
                  onClick={handleDownloadCSV}
                  disabled={downloadingCSV}
                  className="w-full bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded-lg font-semibold transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <i
                    className={`fas ${downloadingCSV ? 'fa-spinner fa-spin' : 'fa-download'} mr-2`}
                  ></i>
                  {downloadingCSV ? 'Downloading...' : 'Download Current Form CSV'}
                </button>
              </div>

              <form onSubmit={handleUpload} className="space-y-6">
                <div>
                  <label className="block text-sm font-semibold text-gray-300 mb-2">
                    <i className="fas fa-file-csv mr-2"></i>Select CSV File
                  </label>
                  <input
                    type="file"
                    accept=".csv"
                    required
                    onChange={(e) => {
                      setSelectedFile(e.target.files?.[0] ?? null)
                      setUploadStatus(null)
                    }}
                    className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white file:mr-4 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-semibold file:bg-blue-600 file:text-white hover:file:bg-blue-700 file:cursor-pointer"
                  />
                  {selectedFile && (
                    <p className="mt-2 text-sm text-gray-400">
                      <i className="fas fa-file mr-2"></i>Selected: {selectedFile.name}
                    </p>
                  )}
                </div>
                <button
                  type="submit"
                  disabled={uploading || !selectedFile}
                  className="w-full bg-green-600 hover:bg-green-700 text-white px-6 py-3 rounded-lg font-semibold transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <i className={`fas ${uploading ? 'fa-spinner fa-spin' : 'fa-cloud-upload-alt'} mr-2`}></i>
                  {uploading ? 'Uploading and processing...' : 'Upload & Generate Schedule'}
                </button>
              </form>

              {uploadStatus && (
                <div
                  className={`mt-4 p-4 rounded-lg ${
                    uploadStatus.type === 'success'
                      ? 'bg-green-900/50 border-l-4 border-green-500 text-green-200'
                      : 'bg-red-900/50 border-l-4 border-red-500 text-red-200'
                  }`}
                >
                  <i
                    className={`fas ${
                      uploadStatus.type === 'success' ? 'fa-check-circle' : 'fa-times-circle'
                    } mr-2`}
                  ></i>
                  {uploadStatus.message}
                </div>
              )}
            </div>
          )}

          {/* Generate Schedule Tab */}
          {activeTab === 'generate-schedule' && (
            <div>
              <div className="text-center mb-8">
                <div className="inline-block bg-green-900/50 rounded-full p-4 mb-4">
                  <i className="fas fa-calendar-alt text-green-400 text-3xl"></i>
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">Generate Schedule</h2>
                <p className="text-gray-400">Generate schedules from form submissions</p>
              </div>

              {!loadingCurrentForm && !currentForm && (
                <div className="bg-yellow-900/50 border-l-4 border-yellow-500 text-yellow-200 p-4 rounded-lg mb-6">
                  <i className="fas fa-exclamation-triangle mr-2"></i>
                  No current form found. Please create a form first.
                </div>
              )}

              <div className="bg-gray-700/50 rounded-lg p-6 border border-gray-600 mb-6">
                <h3 className="text-xl font-bold text-white mb-4">
                  <i className="fas fa-lock mr-2"></i>Predetermined Slots
                </h3>
                <p className="text-sm text-gray-400 mb-4">
                  Pre-assign specific time slots to players. Loaded from current form configuration.
                </p>
                {!currentForm ? (
                  <div className="text-gray-400 text-center py-4">
                    No current form found. Please create a form first.
                  </div>
                ) : (
                  <div className="space-y-3">
                    {predeterminedSlots.map((slot, index) => (
                      <div
                        key={index}
                        className="bg-gray-800 rounded-lg p-4 border border-gray-600"
                      >
                        <div className="grid md:grid-cols-2 gap-3 mb-3">
                          <div>
                            <label className="block text-sm font-semibold text-gray-300 mb-2">
                              Day <span className="text-red-400">*</span>
                            </label>
                            <select
                              value={slot.day}
                              onChange={(e) =>
                                setPredeterminedSlots((prev) => {
                                  const next = [...prev]
                                  next[index] = { ...next[index], day: e.target.value }
                                  return next
                                })
                              }
                              className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                            >
                              <option value="construction">Construction Day</option>
                              <option value="research">Research Day</option>
                              <option value="troops">Troops Training Day</option>
                            </select>
                          </div>
                          <div>
                            <label className="block text-sm font-semibold text-gray-300 mb-2">
                              Time (HH:MM) <span className="text-red-400">*</span>
                            </label>
                            <input
                              type="time"
                              value={slot.time}
                              onChange={(e) =>
                                setPredeterminedSlots((prev) => {
                                  const next = [...prev]
                                  next[index] = { ...next[index], time: e.target.value }
                                  return next
                                })
                              }
                              className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                            />
                          </div>
                          <div>
                            <label className="block text-sm font-semibold text-gray-300 mb-2">
                              Player ID <span className="text-red-400">*</span>
                            </label>
                            <input
                              type="text"
                              value={slot.player_id ?? ''}
                              onBlur={() => lookupPlayer(index)}
                              onChange={(e) =>
                                setPredeterminedSlots((prev) => {
                                  const next = [...prev]
                                  next[index] = { ...next[index], player_id: e.target.value }
                                  return next
                                })
                              }
                              placeholder="Enter player ID"
                              className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
                            />
                            {slot.lookingUp && (
                              <div className="text-xs text-gray-400 mt-1">
                                <i className="fas fa-spinner fa-spin mr-1"></i>Looking up...
                              </div>
                            )}
                            {slot.lookupError && (
                              <div className="text-xs text-red-400 mt-1">
                                <i className="fas fa-exclamation-triangle mr-1"></i>
                                {slot.lookupError}
                              </div>
                            )}
                          </div>
                          <div>
                            <label className="block text-sm font-semibold text-gray-300 mb-2">
                              Alliance
                            </label>
                            <input
                              type="text"
                              value={slot.alliance ?? ''}
                              readOnly
                              disabled
                              className="w-full px-4 py-2 bg-gray-800 border border-gray-600 rounded-lg text-gray-400 cursor-not-allowed"
                              placeholder="Auto-filled from player ID"
                            />
                          </div>
                          <div className="md:col-span-2">
                            <label className="block text-sm font-semibold text-gray-300 mb-2">
                              Player Name
                            </label>
                            <input
                              type="text"
                              value={slot.name ?? ''}
                              readOnly
                              disabled
                              className="w-full px-4 py-2 bg-gray-800 border border-gray-600 rounded-lg text-gray-400 cursor-not-allowed"
                              placeholder="Auto-filled from player ID"
                            />
                          </div>
                        </div>
                        <button
                          onClick={() =>
                            setPredeterminedSlots((prev) => prev.filter((_, i) => i !== index))
                          }
                          className="w-full px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg transition-all"
                        >
                          <i className="fas fa-trash mr-2"></i>Remove Slot
                        </button>
                      </div>
                    ))}
                    <button
                      onClick={() =>
                        setPredeterminedSlots((prev) => [
                          ...prev,
                          {
                            day: 'construction',
                            time: '00:00',
                            player_id: '',
                            alliance: '',
                            name: '',
                            lookingUp: false,
                            lookupError: null,
                          },
                        ])
                      }
                      className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-all"
                    >
                      <i className="fas fa-plus mr-2"></i>Add Predetermined Slot
                    </button>
                  </div>
                )}
              </div>

              <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
                <h3 className="text-lg font-semibold text-white mb-3">Generate all days</h3>
                <div className="flex flex-col sm:flex-row gap-4 mb-6">
                  <button
                    onClick={() => handleGenerateSchedule(false)}
                    disabled={generatingSchedule}
                    className="flex-1 bg-green-600 hover:bg-green-700 text-white px-6 py-3 rounded-lg font-semibold transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    <i
                      className={`fas ${
                        generatingSchedule ? 'fa-spinner fa-spin' : 'fa-magic'
                      } mr-2`}
                    ></i>
                    {generatingSchedule ? 'Generating...' : 'Generate Schedule (Replace)'}
                  </button>
                  <button
                    onClick={() => handleGenerateSchedule(true)}
                    disabled={generatingSchedule}
                    className="flex-1 bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded-lg font-semibold transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    <i
                      className={`fas ${
                        generatingSchedule ? 'fa-spinner fa-spin' : 'fa-plus'
                      } mr-2`}
                    ></i>
                    {generatingSchedule ? 'Generating...' : 'Append to Schedule'}
                  </button>
                </div>
                <p className="mb-6 text-sm text-gray-400">
                  <strong>Replace:</strong> Build a new schedule from scratch.{' '}
                  <strong>Append:</strong> Fill only empty slots, keep current assignments.
                </p>

                <h3 className="text-lg font-semibold text-white mb-3">Generate single day</h3>
                <p className="text-sm text-gray-400 mb-4">
                  Replace or append to one day. Crossover slot (Construction last = Research slot 1) is handled correctly.
                </p>
                <div className="space-y-4">
                  {(Object.keys(SCHEDULE_DAYS) as ScheduleDayKey[]).map((key) => (
                    <div key={key} className="flex flex-wrap items-center gap-2">
                      <span className="text-sm font-medium text-gray-400 w-24 sm:w-28">
                        <i className={`${SCHEDULE_DAYS[key].icon} mr-1.5 text-gray-500`}></i>
                        {SCHEDULE_DAYS[key].name}:
                      </span>
                      <button
                        onClick={() => handleGenerateSchedule(false, key)}
                        disabled={generatingSchedule}
                        className="px-3 py-1.5 bg-gray-600 hover:bg-gray-500 text-white rounded-lg text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        {generatingSchedule ? <i className="fas fa-spinner fa-spin mr-1.5"></i> : null}
                        Replace
                      </button>
                      <button
                        onClick={() => handleGenerateSchedule(true, key)}
                        disabled={generatingSchedule}
                        className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed ${
                          key === 'construction'
                            ? 'bg-orange-600 hover:bg-orange-700 text-white'
                            : key === 'research'
                            ? 'bg-blue-600 hover:bg-blue-700 text-white'
                            : 'bg-green-600 hover:bg-green-700 text-white'
                        }`}
                      >
                        {generatingSchedule ? <i className="fas fa-spinner fa-spin mr-1.5"></i> : <i className="fas fa-plus mr-1.5"></i>}
                        Append
                      </button>
                    </div>
                  ))}
                </div>

                {scheduleGenStatus && (
                  <div
                    className={`mt-4 p-4 rounded-lg ${
                      scheduleGenStatus.type === 'success'
                        ? 'bg-green-900/50 border-l-4 border-green-500 text-green-200'
                        : 'bg-red-900/50 border-l-4 border-red-500 text-red-200'
                    }`}
                  >
                    <i
                      className={`fas ${
                        scheduleGenStatus.type === 'success' ? 'fa-check-circle' : 'fa-times-circle'
                      } mr-2`}
                    ></i>
                    {scheduleGenStatus.message}
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
