import { useCallback, useEffect, useMemo, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { api, FormConfig, FormSubmission, PlayerCard } from '../api/client'
import { tForm, FormLang } from '../i18n/formTranslations'
import { calculateTimeSlots } from '../utils/timeSlots'

const LANG_OPTIONS: { value: FormLang; label: string }[] = [
  { value: 'en', label: 'English' },
  { value: 'ko', label: '한국어' },
  { value: 'zh', label: '中文' },
  { value: 'ja', label: '日本語' },
  { value: 'es', label: 'Español' },
  { value: 'de', label: 'Deutsch' },
  { value: 'fr', label: 'Français' },
]

const NON_OF_ABOVE = 'Non of the above'

type SubmissionType = 'New submission' | 'Re-Submission'

const initialForm = {
  alliance: '',
  custom_alliance: '',
  character_name: '',
  player_id: '',
  submission_type: 'New submission' as SubmissionType,
  wants_construction: false,
  construction_speedups: undefined as number | undefined,
  construction_truegold: undefined as number | undefined,
  construction_tempered_truegold: undefined as number | undefined,
  construction_time_slots: [] as number[],
  wants_research: false,
  research_speedups: undefined as number | undefined,
  research_truegold_dust: undefined as number | undefined,
  research_time_slots: [] as number[],
  wants_troops: false,
  troops_speedups: undefined as number | undefined,
  troops_time_slots: [] as number[],
  additional_notes: '',
  suggestions: '',
}

export default function FormPage() {
  const { code } = useParams<{ code: string }>()
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [config, setConfig] = useState<FormConfig | null>(null)
  const [form, setForm] = useState(initialForm)
  const [submitted, setSubmitted] = useState(false)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [submitError, setSubmitError] = useState('')
  const [lang, setLang] = useState<FormLang>(() => (localStorage.getItem('form_language') as FormLang) || 'en')
  const [lookupLoading, setLookupLoading] = useState(false)
  const [lookupError, setLookupError] = useState('')
  const [playerCard, setPlayerCard] = useState<PlayerCard | null>(null)
  const [checkLoading, setCheckLoading] = useState(false)

  const baseUrl = `/form/${code}`

  const constructionSlots = useMemo(() => {
    const ct = config?.construction_times
    return calculateTimeSlots(ct?.start_time ?? '00:00', ct?.end_time ?? null)
  }, [config?.construction_times])

  const researchSlots = useMemo(() => {
    const rt = config?.research_times
    return calculateTimeSlots(rt?.start_time ?? '00:00', rt?.end_time ?? null)
  }, [config?.research_times])

  const troopsSlots = useMemo(() => {
    const tt = config?.troops_times
    return calculateTimeSlots(tt?.start_time ?? '00:00', tt?.end_time ?? null)
  }, [config?.troops_times])

  const t = useCallback((key: string, params?: Record<string, string | number>) => tForm(lang, key, params), [lang])

  const tgMode = config?.construction_truegold_mode ?? 'truegold_unlocked'
  const showConstructionTruegold = ['truegold_unlocked', 'war_academy_unlocked', 'tempered_truegold_unlocked'].includes(tgMode)
  const showConstructionTempered = tgMode === 'tempered_truegold_unlocked'
  const showResearchTruegoldDust = ['war_academy_unlocked', 'tempered_truegold_unlocked'].includes(tgMode)

  const alliances = useMemo(() => {
    const list = [...(config?.alliances ?? [])]
    if (config?.include_non_of_above !== false && !list.includes(NON_OF_ABOVE)) {
      list.push(NON_OF_ABOVE)
    }
    return list
  }, [config])

  useEffect(() => {
    if (!code) return
    setLoading(true)
    setError(null)
    api.getFormConfig(code).then(({ ok, data, error: err }) => {
      if (ok && data) setConfig(data)
      else setError(err || t('formNotFound'))
      setLoading(false)
    })
  }, [code])

  const checkSubmission = useCallback(async () => {
    const id = form.player_id.trim()
    if (!id || !/^[0-9]+$/.test(id)) {
      setForm((f) => ({ ...f, submission_type: 'New submission' }))
      return
    }
    setCheckLoading(true)
    const { ok, data } = await api.checkSubmission(code!, id)
    setForm((f) => ({ ...f, submission_type: (ok && data?.has_submitted ? 'Re-Submission' : 'New submission') as SubmissionType }))
    setCheckLoading(false)
  }, [code, form.player_id])

  const lookupPlayer = useCallback(async () => {
    const id = form.player_id.trim()
    if (!id || !/^[0-9]+$/.test(id)) {
      setLookupError(t('playerIdMustBeNumber'))
      return
    }
    setLookupLoading(true)
    setLookupError('')
    setPlayerCard(null)
    const { ok, data } = await api.playerLookup(code!, id)
    const d = data as { success?: boolean; name?: string; player_id?: string; kingdom_mismatch?: boolean; error?: string; castle_level?: number | string; kingdom?: string; avatar_image?: string }
    if (ok && d?.success && d?.name) {
      setForm((f) => ({ ...f, character_name: d.name ?? '' }))
      setPlayerCard({
        player_id: d.player_id ?? id,
        name: d.name,
        castle_level: d.castle_level != null ? String(d.castle_level) : undefined,
        kingdom: d.kingdom,
        avatar_image: d.avatar_image,
      })
      checkSubmission()
    } else {
      setLookupError(d?.kingdom_mismatch ? t('playerNotInKingdom') : (d?.error ?? t('failedToLoadConfig')))
    }
    setLookupLoading(false)
  }, [code, form.player_id, checkSubmission, t])

  const handleAllianceChange = (alliance: string) => {
    setForm((f) => ({ ...f, alliance, custom_alliance: alliance === NON_OF_ABOVE ? f.custom_alliance : '' }))
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setSubmitError('')
    if (!form.alliance) {
      setSubmitError(t('pleaseSelectAlliance'))
      return
    }
    if (form.alliance === NON_OF_ABOVE && !form.custom_alliance?.trim()) {
      setSubmitError(t('pleaseEnterCustomAlliance'))
      return
    }
    if (!form.character_name.trim()) {
      setSubmitError(t('pleaseEnterCharacterName'))
      return
    }
    if (!/^[0-9]+$/.test(form.player_id)) {
      setSubmitError(t('playerIdMustBeNumber'))
      return
    }
    if (!form.wants_construction && !form.wants_research && !form.wants_troops) {
      setSubmitError(t('pleaseSelectAtLeastOneDay'))
      return
    }
    if (form.wants_construction) {
      if ((form.construction_speedups ?? 0) < 0) {
        setSubmitError(t('pleaseEnterConstructionSpeedups'))
        return
      }
      if (showConstructionTruegold && (form.construction_truegold ?? 0) < 0) {
        setSubmitError(t('pleaseEnterConstructionTruegold'))
        return
      }
      if (showConstructionTempered && (form.construction_tempered_truegold ?? 0) < 0) {
        setSubmitError(t('pleaseEnterConstructionTemperedTruegold'))
        return
      }
      if (form.construction_time_slots.length < 5) {
        setSubmitError(t('pleaseSelectConstructionTimeSlots'))
        return
      }
    }
    if (form.wants_research) {
      if ((form.research_speedups ?? 0) < 0) {
        setSubmitError(t('pleaseEnterResearchSpeedups'))
        return
      }
      if (showResearchTruegoldDust && (form.research_truegold_dust ?? 0) < 0) {
        setSubmitError(t('pleaseEnterResearchTruegoldDust'))
        return
      }
      if (form.research_time_slots.length < 5) {
        setSubmitError(t('pleaseSelectResearchTimeSlots'))
        return
      }
    }
    if (form.wants_troops) {
      if ((form.troops_speedups ?? 0) < 0) {
        setSubmitError(t('pleaseEnterTroopsSpeedups'))
        return
      }
      if (form.troops_time_slots.length < 5) {
        setSubmitError(t('pleaseSelectTroopsTimeSlots'))
        return
      }
    }

    const payload: FormSubmission = {
      alliance: form.alliance,
      custom_alliance: form.alliance === NON_OF_ABOVE ? form.custom_alliance || undefined : undefined,
      character_name: form.character_name,
      player_id: form.player_id,
      submission_type: form.submission_type,
      wants_construction: form.wants_construction,
      construction_speedups: form.wants_construction ? (form.construction_speedups ?? 0) : undefined,
      construction_truegold: form.wants_construction && showConstructionTruegold ? (form.construction_truegold ?? 0) : undefined,
      construction_tempered_truegold: form.wants_construction && showConstructionTempered ? (form.construction_tempered_truegold ?? 0) : undefined,
      construction_time_slots: form.wants_construction ? form.construction_time_slots : [],
      wants_research: form.wants_research,
      research_speedups: form.wants_research ? (form.research_speedups ?? 0) : undefined,
      research_truegold_dust: form.wants_research && showResearchTruegoldDust ? (form.research_truegold_dust ?? 0) : undefined,
      research_time_slots: form.wants_research ? form.research_time_slots : [],
      wants_troops: form.wants_troops,
      troops_speedups: form.wants_troops ? (form.troops_speedups ?? 0) : undefined,
      troops_time_slots: form.wants_troops ? form.troops_time_slots : [],
      additional_notes: form.additional_notes || undefined,
      suggestions: form.suggestions || undefined,
    }

    setIsSubmitting(true)
    const { ok, error: err } = await api.submitForm(code!, payload)
    setIsSubmitting(false)
    if (ok) {
      setSubmitted(true)
    } else {
      setSubmitError(err ?? t('failedToSubmitForm'))
    }
  }

  const resetForm = () => {
    setForm(initialForm)
    setSubmitted(false)
    setPlayerCard(null)
    setLookupError('')
  }

  const changeLang = (l: FormLang) => {
    setLang(l)
    localStorage.setItem('form_language', l)
  }

  const displayIntroText = useMemo(() => {
    if (!config) return ''

    // Determine logical day slots for construction/research based on config (fallback to defaults if missing)
    const constructionSlot = (config.construction_day_slot as string | undefined) ?? 'monday'
    const researchSlot = (config.research_day_slot as string | undefined) ?? 'tuesday'

    const constructionDayKey = `scheduleDay_${constructionSlot}`
    const researchDayKey = `scheduleDay_${researchSlot}`
    const troopsDayKey = 'scheduleDay_thursday'

    const constructionDay = t(constructionDayKey)
    const researchDay = t(researchDayKey)
    const troopsDay = t(troopsDayKey)

    const support = config.support_person_name?.trim() || '#140 [COB]Vor'

    const parts = [
      t('introHeader'),
      '',
      t('introScheduleHeading'),
      t('introScheduleLineConstruction', { day: constructionDay }),
      t('introScheduleLineResearch', { day: researchDay }),
      t('introScheduleLineTroops', { day: troopsDay }),
      '',
      t('introRequirements'),
      '',
      t('introMoreInfo', { support }),
    ]

    return parts.join('\n')
  }, [config, t])

  const isFridaySatSlot = (slotId?: string | null) => slotId === 'friday_sat'

  const getDayTagLabel = (slotId?: string | null) => {
    if (!slotId) return ''
    const key = `scheduleDay_${slotId}` as
      | 'scheduleDay_monday'
      | 'scheduleDay_tuesday'
      | 'scheduleDay_thursday'
      | 'scheduleDay_friday_full'
      | 'scheduleDay_friday_sat'
    const full = t(key)
    const [dayWord] = full.split(' ')
    return dayWord || full
  }

  const splitFridaySaturday = (slots: Array<{ value: number; label: string }>) => {
    const friday: typeof slots = []
    const saturday: typeof slots = []
    for (const slot of slots) {
      const [hStr] = slot.label.split(':')
      const h = parseInt(hStr, 10) || 0
      if (h >= 10) friday.push(slot)
      else saturday.push(slot)
    }
    return { friday, saturday }
  }

  if (!code) {
    return (
      <div className="container mx-auto px-4 py-8">
        <p className="text-red-400">Invalid form code</p>
        <Link to="/" className="text-blue-400 mt-4 inline-block">← Home</Link>
      </div>
    )
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      <header className="text-center mb-12">
        <h1 className="text-4xl font-bold text-blue-400 mb-4">
          <i className="fas fa-calendar-check mr-3"></i> {t('submitAppointmentForm')}
        </h1>
        <p className="text-gray-400">{t('fillOutFormDescription')}</p>
      </header>

      <div className="flex justify-end mb-4">
        <select
          value={lang}
          onChange={(e) => changeLang(e.target.value as FormLang)}
          className="px-4 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
        >
          {LANG_OPTIONS.map((o) => (
            <option key={o.value} value={o.value}>{o.label}</option>
          ))}
        </select>
      </div>

      <nav className="flex justify-center gap-4 mb-12 flex-wrap">
        <Link to={`${baseUrl}/stats`} className="px-6 py-3 bg-gray-800 hover:bg-gray-700 text-white rounded-lg font-medium transition-all border border-gray-700">
          <i className="fas fa-chart-bar mr-2"></i> {t('statistics')}
        </Link>
        <Link to={baseUrl} className="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-all border border-blue-500">
          <i className="fas fa-edit mr-2"></i> {t('submitForm')}
        </Link>
      </nav>

      <main>
        {loading && (
          <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
            <i className="fas fa-spinner fa-spin text-4xl text-blue-400 mb-4"></i>
            <p className="text-xl text-gray-400">{t('loadingFormConfiguration')}</p>
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

        {!loading && !error && config && !submitted && (
          <>
            {config.intro_text && (
              <div className="bg-blue-900/30 border-l-4 border-blue-500 rounded-lg p-6 mb-8">
                <div className="whitespace-pre-line text-gray-200">{displayIntroText}</div>
              </div>
            )}

            <form onSubmit={handleSubmit} className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700 space-y-8">
              {/* Basic Information */}
              <div className="border-b border-gray-700 pb-6">
                <h2 className="text-2xl font-bold text-blue-400 mb-6">
                  <i className="fas fa-user mr-2"></i>{t('basicInformation')}
                </h2>
                <div className="space-y-6">
                  <div>
                    <label className="block text-sm font-semibold text-gray-300 mb-2">
                      {t('allianceQuestion')} <span className="text-red-400">*</span>
                    </label>
                    <select
                      value={form.alliance}
                      onChange={(e) => handleAllianceChange(e.target.value)}
                      required
                      className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                    >
                      <option value="">--</option>
                      {alliances.map((a) => (
                        <option key={a} value={a}>{a === NON_OF_ABOVE ? t('nonOfAbove') : a}</option>
                      ))}
                    </select>
                  </div>

                  {form.alliance === NON_OF_ABOVE && (
                    <div>
                      <label className="block text-sm font-semibold text-gray-300 mb-2">
                        {t('customAllianceLabel')} <span className="text-red-400">*</span>
                      </label>
                      <input
                        type="text"
                        value={form.custom_alliance}
                        onChange={(e) => setForm((f) => ({ ...f, custom_alliance: e.target.value }))}
                        required
                        className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                        placeholder={t('customAlliancePlaceholder')}
                      />
                    </div>
                  )}

                  <div>
                    <label className="block text-sm font-semibold text-gray-300 mb-2">
                      {t('playerIdQuestion')} <span className="text-red-400">*</span>
                    </label>
                    <p className="text-xs text-gray-500 mb-2">{t('playerIdNote')}</p>
                    <div className="flex gap-2">
                      <input
                        type="text"
                        value={form.player_id}
                        onChange={(e) => {
                          setForm((f) => ({ ...f, player_id: e.target.value }))
                          setPlayerCard(null)
                          setLookupError('')
                        }}
                        onBlur={checkSubmission}
                        required
                        pattern="[0-9]+"
                        className="flex-1 px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                        placeholder={t('playerIdPlaceholder')}
                      />
                      <button
                        type="button"
                        onClick={lookupPlayer}
                        disabled={lookupLoading || !form.player_id.trim()}
                        className="px-4 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-all whitespace-nowrap"
                      >
                        {lookupLoading ? <i className="fas fa-spinner fa-spin mr-2"></i> : <i className="fas fa-check mr-2"></i>}
                        {lookupLoading ? t('lookingUp') : t('confirm')}
                      </button>
                    </div>
                    {lookupError && <p className="text-xs text-red-400 mt-2">{lookupError}</p>}
                    {playerCard && (
                      <div className="mt-4 p-4 bg-gray-800/80 rounded-lg border border-dashed border-gray-500">
                        <div className="flex items-start gap-4">
                          <div className="flex-shrink-0">
                            {playerCard.avatar_image ? (
                              <img src={playerCard.avatar_image} alt="" className="w-16 h-16 rounded-full object-cover border-2 border-gray-500" />
                            ) : (
                              <div className="w-16 h-16 rounded-full bg-gray-600 flex items-center justify-center border-2 border-gray-500">
                                <i className="fas fa-user text-2xl text-gray-400"></i>
                              </div>
                            )}
                          </div>
                          <div className="flex-1 min-w-0">
                            <div className="space-y-1 text-sm text-gray-300">
                              {playerCard.name && (
                                <p className="font-semibold text-white"><i className="fas fa-user text-blue-400 w-4 mr-2"></i>{playerCard.name}</p>
                              )}
                              <p><i className="fas fa-id-card text-purple-400 w-4 mr-2"></i>ID: {playerCard.player_id}</p>
                              {playerCard.castle_level != null && playerCard.castle_level !== '' && (
                                <p><i className="fas fa-chess-rook text-amber-400 w-4 mr-2"></i> {t('castleLevel')}: {playerCard.castle_level}</p>
                              )}
                              {playerCard.kingdom && <p><i className="fas fa-globe text-blue-400 w-4 mr-2"></i>{t('kingdom')}: {playerCard.kingdom}</p>}
                            </div>
                          </div>
                        </div>
                      </div>
                    )}
                  </div>

                  <div>
                    <label className="block text-sm font-semibold text-gray-300 mb-2">{t('submissionType')}</label>
                    <div className="px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white">
                      {checkLoading ? (
                        <span className="text-gray-400"><i className="fas fa-spinner fa-spin mr-2"></i>{t('checkingSubmission')}</span>
                      ) : (
                        <span className="font-medium">
                          {form.submission_type === 'Re-Submission' ? t('updateSubmission') : t('newSubmission')}
                          {form.player_id.trim() && <span className="text-xs text-gray-400 ml-2">({t('autoDetected')})</span>}
                        </span>
                      )}
                    </div>
                  </div>
                </div>
              </div>

              {/* Construction Day */}
              <div className="border-b border-gray-700 pb-6">
                <h2 className="text-2xl font-bold text-orange-400 mb-6">
                  <i className="fas fa-hammer mr-2"></i>{t('constructionDay')}
                </h2>
                <div className="space-y-6">
                  <label className="flex items-center cursor-pointer">
                    <input
                      type="checkbox"
                      checked={form.wants_construction}
                      onChange={(e) => setForm((f) => ({ ...f, wants_construction: e.target.checked }))}
                      className="w-5 h-5 text-blue-600 bg-gray-700 border-gray-600 rounded focus:ring-blue-500 focus:ring-2"
                    />
                    <span className="ml-3 text-gray-300 font-medium">{t('wantsConstruction')}</span>
                  </label>
                  {form.wants_construction && (
                    <div className="space-y-4 pl-8 border-l-2 border-orange-500/30">
                      <div>
                        <label className="block text-sm font-semibold text-gray-300 mb-2">
                          {t('constructionSpeedupsLabel')} <img src="/static/icons/Speedups.png" alt="" className="inline-block w-5 h-5 ml-1 align-middle" /> <span className="text-red-400">*</span>
                        </label>
                        <p className="text-xs text-gray-500 mb-2">{t('constructionSpeedupsNote')}</p>
                        <input
                          type="number"
                          min={0}
                          value={form.construction_speedups ?? ''}
                          onChange={(e) => setForm((f) => ({ ...f, construction_speedups: e.target.value === '' ? undefined : parseInt(e.target.value, 10) }))}
                          required
                          className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                          placeholder={t('constructionSpeedupsPlaceholder')}
                        />
                      </div>
                      {showConstructionTruegold && (
                        <div>
                          <label className="block text-sm font-semibold text-gray-300 mb-2">
                            {t('constructionTruegoldLabel')} <img src="/static/icons/Truegold.png" alt="" className="inline-block w-5 h-5 ml-1 align-middle" /> <span className="text-red-400">*</span>
                          </label>
                          <input
                            type="number"
                            min={0}
                            value={form.construction_truegold ?? ''}
                            onChange={(e) => setForm((f) => ({ ...f, construction_truegold: e.target.value === '' ? undefined : parseInt(e.target.value, 10) }))}
                            required
                            className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                            placeholder={t('constructionTruegoldPlaceholder')}
                          />
                        </div>
                      )}
                      {showConstructionTempered && (
                        <div>
                          <label className="block text-sm font-semibold text-gray-300 mb-2">
                            {t('constructionTemperedTruegoldLabel')} <img src="/static/icons/TamperedTruegold.png" alt="" className="inline-block w-5 h-5 ml-1 align-middle" /> <span className="text-red-400">*</span>
                          </label>
                          <input
                            type="number"
                            min={0}
                            value={form.construction_tempered_truegold ?? ''}
                            onChange={(e) => setForm((f) => ({ ...f, construction_tempered_truegold: e.target.value === '' ? undefined : parseInt(e.target.value, 10) }))}
                            required
                            className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                            placeholder={t('constructionTemperedTruegoldPlaceholder')}
                          />
                        </div>
                      )}
                      <div>
                        <label className="block text-sm font-semibold text-gray-300 mb-2">{t('constructionTimeSlotsLabel')} <span className="text-red-400">*</span></label>
                        <p className="text-xs text-gray-500 mb-4">{t('constructionTimeSlotsNote')}</p>
                        {isFridaySatSlot(config?.construction_day_slot as string | undefined) ? (
                          <div className="space-y-4 bg-gray-900/50 p-4 rounded-lg">
                            {(() => {
                              const { friday, saturday } = splitFridaySaturday(constructionSlots)
                              return (
                                <>
                                  {friday.length > 0 && (
                                    <div>
                                      <p className="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-2">
                                        Friday
                                      </p>
                                      <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 gap-2">
                                        {friday.map((slot) => (
                                          <label key={slot.value} className="flex items-center space-x-2 cursor-pointer hover:bg-gray-700/50 p-2 rounded">
                                            <input
                                              type="checkbox"
                                              checked={form.construction_time_slots.includes(slot.value)}
                                              onChange={(e) => {
                                                const v = slot.value
                                                setForm((f) => ({
                                                  ...f,
                                                  construction_time_slots: e.target.checked
                                                    ? [...f.construction_time_slots, v].sort((a, b) => a - b)
                                                    : f.construction_time_slots.filter((x) => x !== v),
                                                }))
                                              }}
                                              className="w-4 h-4 text-blue-600 bg-gray-700 border-gray-600 rounded focus:ring-blue-500"
                                            />
                                            <span className="text-sm text-gray-300">{slot.label}</span>
                                          </label>
                                        ))}
                                      </div>
                                    </div>
                                  )}
                                  {saturday.length > 0 && (
                                    <div>
                                      <p className="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-2">
                                        Saturday
                                      </p>
                                      <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 gap-2">
                                        {saturday.map((slot) => (
                                          <label key={slot.value} className="flex items-center space-x-2 cursor-pointer hover:bg-gray-700/50 p-2 rounded">
                                            <input
                                              type="checkbox"
                                              checked={form.construction_time_slots.includes(slot.value)}
                                              onChange={(e) => {
                                                const v = slot.value
                                                setForm((f) => ({
                                                  ...f,
                                                  construction_time_slots: e.target.checked
                                                    ? [...f.construction_time_slots, v].sort((a, b) => a - b)
                                                    : f.construction_time_slots.filter((x) => x !== v),
                                                }))
                                              }}
                                              className="w-4 h-4 text-blue-600 bg-gray-700 border-gray-600 rounded focus:ring-blue-500"
                                            />
                                            <span className="text-sm text-gray-300">{slot.label}</span>
                                          </label>
                                        ))}
                                      </div>
                                    </div>
                                  )}
                                </>
                              )
                            })()}
                          </div>
                        ) : (
                          <div className="space-y-2 bg-gray-900/50 p-4 rounded-lg">
                            <p className="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-1">
                              {getDayTagLabel(config?.construction_day_slot as string | null)}
                            </p>
                            <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 gap-2">
                              {constructionSlots.map((slot) => (
                                <label key={slot.value} className="flex items-center space-x-2 cursor-pointer hover:bg-gray-700/50 p-2 rounded">
                                  <input
                                    type="checkbox"
                                    checked={form.construction_time_slots.includes(slot.value)}
                                    onChange={(e) => {
                                      const v = slot.value
                                      setForm((f) => ({
                                        ...f,
                                        construction_time_slots: e.target.checked
                                          ? [...f.construction_time_slots, v].sort((a, b) => a - b)
                                          : f.construction_time_slots.filter((x) => x !== v),
                                      }))
                                    }}
                                    className="w-4 h-4 text-blue-600 bg-gray-700 border-gray-600 rounded focus:ring-blue-500"
                                  />
                                  <span className="text-sm text-gray-300">{slot.label}</span>
                                </label>
                              ))}
                            </div>
                          </div>
                        )}
                        {form.construction_time_slots.length < 5 && (
                          <p className="text-xs text-red-400 mt-2">{t('constructionTimeSlotsError', { count: form.construction_time_slots.length })}</p>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              </div>

              {/* Research Day */}
              <div className="border-b border-gray-700 pb-6">
                <h2 className="text-2xl font-bold text-purple-400 mb-6">
                  <i className="fas fa-flask mr-2"></i>{t('researchDay')}
                </h2>
                <div className="space-y-6">
                  <label className="flex items-center cursor-pointer">
                    <input
                      type="checkbox"
                      checked={form.wants_research}
                      onChange={(e) => setForm((f) => ({ ...f, wants_research: e.target.checked }))}
                      className="w-5 h-5 text-blue-600 bg-gray-700 border-gray-600 rounded focus:ring-blue-500 focus:ring-2"
                    />
                    <span className="ml-3 text-gray-300 font-medium">{t('wantsResearch')}</span>
                  </label>
                  {form.wants_research && (
                    <div className="space-y-4 pl-8 border-l-2 border-purple-500/30">
                      <div>
                        <label className="block text-sm font-semibold text-gray-300 mb-2">
                          {t('researchSpeedupsLabel')} <img src="/static/icons/Speedups.png" alt="" className="inline-block w-5 h-5 ml-1 align-middle" /> <span className="text-red-400">*</span>
                        </label>
                        <p className="text-xs text-gray-500 mb-2">{t('researchSpeedupsNote')}</p>
                        <input
                          type="number"
                          min={0}
                          value={form.research_speedups ?? ''}
                          onChange={(e) => setForm((f) => ({ ...f, research_speedups: e.target.value === '' ? undefined : parseInt(e.target.value, 10) }))}
                          required
                          className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                          placeholder={t('researchSpeedupsPlaceholder')}
                        />
                      </div>
                      {showResearchTruegoldDust && (
                        <div>
                          <label className="block text-sm font-semibold text-gray-300 mb-2">
                            {t('researchTruegoldDustLabel')} <img src="/static/icons/TruegoldDust.png" alt="" className="inline-block w-5 h-5 ml-1 align-middle" /> <span className="text-red-400">*</span>
                          </label>
                          <input
                            type="number"
                            min={0}
                            value={form.research_truegold_dust ?? ''}
                            onChange={(e) => setForm((f) => ({ ...f, research_truegold_dust: e.target.value === '' ? undefined : parseInt(e.target.value, 10) }))}
                            required
                            className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                            placeholder={t('researchTruegoldDustPlaceholder')}
                          />
                        </div>
                      )}
                      <div>
                        <label className="block text-sm font-semibold text-gray-300 mb-2">{t('researchTimeSlotsLabel')} <span className="text-red-400">*</span></label>
                        <p className="text-xs text-gray-500 mb-4">{t('researchTimeSlotsNote')}</p>
                        {isFridaySatSlot(config?.research_day_slot as string | undefined) ? (
                          <div className="space-y-4 bg-gray-900/50 p-4 rounded-lg">
                            {(() => {
                              const { friday, saturday } = splitFridaySaturday(researchSlots)
                              return (
                                <>
                                  {friday.length > 0 && (
                                    <div>
                                      <p className="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-2">
                                        Friday
                                      </p>
                                      <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 gap-2">
                                        {friday.map((slot) => (
                                          <label key={slot.value} className="flex items-center space-x-2 cursor-pointer hover:bg-gray-700/50 p-2 rounded">
                                            <input
                                              type="checkbox"
                                              checked={form.research_time_slots.includes(slot.value)}
                                              onChange={(e) => {
                                                const v = slot.value
                                                setForm((f) => ({
                                                  ...f,
                                                  research_time_slots: e.target.checked
                                                    ? [...f.research_time_slots, v].sort((a, b) => a - b)
                                                    : f.research_time_slots.filter((x) => x !== v),
                                                }))
                                              }}
                                              className="w-4 h-4 text-blue-600 bg-gray-700 border-gray-600 rounded focus:ring-blue-500"
                                            />
                                            <span className="text-sm text-gray-300">{slot.label}</span>
                                          </label>
                                        ))}
                                      </div>
                                    </div>
                                  )}
                                  {saturday.length > 0 && (
                                    <div>
                                      <p className="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-2">
                                        Saturday
                                      </p>
                                      <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 gap-2">
                                        {saturday.map((slot) => (
                                          <label key={slot.value} className="flex items-center space-x-2 cursor-pointer hover:bg-gray-700/50 p-2 rounded">
                                            <input
                                              type="checkbox"
                                              checked={form.research_time_slots.includes(slot.value)}
                                              onChange={(e) => {
                                                const v = slot.value
                                                setForm((f) => ({
                                                  ...f,
                                                  research_time_slots: e.target.checked
                                                    ? [...f.research_time_slots, v].sort((a, b) => a - b)
                                                    : f.research_time_slots.filter((x) => x !== v),
                                                }))
                                              }}
                                              className="w-4 h-4 text-blue-600 bg-gray-700 border-gray-600 rounded focus:ring-blue-500"
                                            />
                                            <span className="text-sm text-gray-300">{slot.label}</span>
                                          </label>
                                        ))}
                                      </div>
                                    </div>
                                  )}
                                </>
                              )
                            })()}
                          </div>
                        ) : (
                          <div className="space-y-2 bg-gray-900/50 p-4 rounded-lg">
                            <p className="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-1">
                              {getDayTagLabel(config?.research_day_slot as string | null)}
                            </p>
                            <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 gap-2">
                              {researchSlots.map((slot) => (
                                <label key={slot.value} className="flex items-center space-x-2 cursor-pointer hover:bg-gray-700/50 p-2 rounded">
                                  <input
                                    type="checkbox"
                                    checked={form.research_time_slots.includes(slot.value)}
                                    onChange={(e) => {
                                      const v = slot.value
                                      setForm((f) => ({
                                        ...f,
                                        research_time_slots: e.target.checked
                                          ? [...f.research_time_slots, v].sort((a, b) => a - b)
                                          : f.research_time_slots.filter((x) => x !== v),
                                      }))
                                    }}
                                    className="w-4 h-4 text-blue-600 bg-gray-700 border-gray-600 rounded focus:ring-blue-500"
                                  />
                                  <span className="text-sm text-gray-300">{slot.label}</span>
                                </label>
                              ))}
                            </div>
                          </div>
                        )}
                        {form.research_time_slots.length < 5 && (
                          <p className="text-xs text-red-400 mt-2">{t('researchTimeSlotsError', { count: form.research_time_slots.length })}</p>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              </div>

              {/* Troops Day */}
              <div className="border-b border-gray-700 pb-6">
                <h2 className="text-2xl font-bold text-green-400 mb-6">
                  <i className="fas fa-users mr-2"></i>{t('troopsTrainingDay')}
                </h2>
                <div className="space-y-6">
                  <label className="flex items-center cursor-pointer">
                    <input
                      type="checkbox"
                      checked={form.wants_troops}
                      onChange={(e) => setForm((f) => ({ ...f, wants_troops: e.target.checked }))}
                      className="w-5 h-5 text-blue-600 bg-gray-700 border-gray-600 rounded focus:ring-blue-500 focus:ring-2"
                    />
                    <span className="ml-3 text-gray-300 font-medium">{t('wantsTroops')}</span>
                  </label>
                  {form.wants_troops && (
                    <div className="space-y-4 pl-8 border-l-2 border-green-500/30">
                      <div>
                        <label className="block text-sm font-semibold text-gray-300 mb-2">
                          {t('troopsSpeedupsLabel')} <img src="/static/icons/Speedups.png" alt="" className="inline-block w-5 h-5 ml-1 align-middle" /> <span className="text-red-400">*</span>
                        </label>
                        <p className="text-xs text-gray-500 mb-2">{t('troopsSpeedupsNote')}</p>
                        <input
                          type="number"
                          min={0}
                          value={form.troops_speedups ?? ''}
                          onChange={(e) => setForm((f) => ({ ...f, troops_speedups: e.target.value === '' ? undefined : parseInt(e.target.value, 10) }))}
                          required
                          className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                          placeholder={t('troopsSpeedupsPlaceholder')}
                        />
                      </div>
                      <div>
                        <label className="block text-sm font-semibold text-gray-300 mb-2">{t('troopsTimeSlotsLabel')} <span className="text-red-400">*</span></label>
                        <p className="text-xs text-gray-500 mb-4">{t('troopsTimeSlotsNote')}</p>
                        <div className="space-y-2 bg-gray-900/50 p-4 rounded-lg">
                          <p className="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-1">
                            {getDayTagLabel('thursday')}
                          </p>
                          <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 gap-2">
                            {troopsSlots.map((slot) => (
                              <label key={slot.value} className="flex items-center space-x-2 cursor-pointer hover:bg-gray-700/50 p-2 rounded">
                                <input
                                  type="checkbox"
                                  checked={form.troops_time_slots.includes(slot.value)}
                                  onChange={(e) => {
                                    const v = slot.value
                                    setForm((f) => ({
                                      ...f,
                                      troops_time_slots: e.target.checked
                                        ? [...f.troops_time_slots, v].sort((a, b) => a - b)
                                        : f.troops_time_slots.filter((x) => x !== v),
                                    }))
                                  }}
                                  className="w-4 h-4 text-blue-600 bg-gray-700 border-gray-600 rounded focus:ring-blue-500"
                                />
                                <span className="text-sm text-gray-300">{slot.label}</span>
                              </label>
                            ))}
                          </div>
                        </div>
                        {form.troops_time_slots.length < 5 && (
                          <p className="text-xs text-red-400 mt-2">{t('troopsTimeSlotsError', { count: form.troops_time_slots.length })}</p>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              </div>

              {/* Additional Fields */}
              <div className="space-y-6">
                <div>
                  <label className="block text-sm font-semibold text-gray-300 mb-2">{t('additionalNotesLabel')}</label>
                  <textarea
                    value={form.additional_notes}
                    onChange={(e) => setForm((f) => ({ ...f, additional_notes: e.target.value }))}
                    rows={4}
                    className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                    placeholder={t('additionalNotesPlaceholder')}
                  />
                </div>
                <div>
                  <label className="block text-sm font-semibold text-gray-300 mb-2">{t('suggestionsLabel')}</label>
                  <textarea
                    value={form.suggestions}
                    onChange={(e) => setForm((f) => ({ ...f, suggestions: e.target.value }))}
                    rows={4}
                    className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                    placeholder={t('suggestionsPlaceholder')}
                  />
                </div>
              </div>

              <div className="flex justify-center pt-6">
                <button
                  type="submit"
                  disabled={isSubmitting}
                  className="px-8 py-4 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded-lg font-semibold text-lg transition-all shadow-lg hover:shadow-xl"
                >
                  {isSubmitting ? (
                    <>
                      <i className="fas fa-spinner fa-spin mr-2"></i>
                      {t('submitting')}
                    </>
                  ) : (
                    <>
                      <i className="fas fa-paper-plane mr-2"></i>
                      {t('submitButton')}
                    </>
                  )}
                </button>
              </div>

              {submitError && (
                <div className="mt-4 p-4 bg-red-900/50 border border-red-500 rounded-lg text-red-200">
                  <i className="fas fa-exclamation-circle mr-2"></i> {submitError}
                </div>
              )}
            </form>
          </>
        )}

        {!loading && !error && config && submitted && (
          <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700 text-center">
            <div className="inline-block bg-green-900/50 rounded-full p-6 mb-6">
              <i className="fas fa-check-circle text-green-400 text-5xl"></i>
            </div>
            <h2 className="text-3xl font-bold text-green-400 mb-4">{t('formSubmittedSuccessfully')}</h2>
            <p className="text-gray-300 mb-8">{t('formSubmittedMessage')}</p>
            <button
              onClick={resetForm}
              className="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-semibold transition-all"
            >
              <i className="fas fa-plus mr-2"></i> {t('submitAnotherForm')}
            </button>
          </div>
        )}
      </main>
    </div>
  )
}
