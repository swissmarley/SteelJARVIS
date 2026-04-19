import Foundation
import Speech
import AVFoundation
import CoreFoundation

// MARK: - Callback types (C function pointers)

public typealias ResultCallback = @convention(c) (UnsafePointer<CChar>, Bool) -> Void
public typealias ErrorCallback = @convention(c) (UnsafePointer<CChar>) -> Void

// MARK: - NativeSpeechRecognizer

public class NativeSpeechRecognizer: NSObject, SFSpeechRecognizerDelegate {
    private var recognizer: SFSpeechRecognizer?
    private var recognitionRequest: SFSpeechAudioBufferRecognitionRequest?
    private var recognitionTask: SFSpeechRecognitionTask?
    private var audioEngine: AVAudioEngine?
    private var onResult: ResultCallback?
    private var onError: ErrorCallback?
    private var isListening: Bool = false
    // Silence detector: SFSpeechRecognizer emits partials continuously but only
    // fires isFinal=true when the input stream ends. We force it by calling
    // endAudio() after `silenceThreshold` seconds of unchanged transcription.
    private var lastRecognizedText: String = ""
    private var silenceTimer: Timer?
    private let silenceThreshold: TimeInterval = 1.2

    public override init() {
        super.init()
        recognizer = SFSpeechRecognizer(locale: Locale(identifier: "en-US"))
        recognizer?.delegate = self
    }

    public func startListening(
        onResult: @escaping ResultCallback,
        onError: @escaping ErrorCallback
    ) {
        if isListening {
            return
        }

        self.onResult = onResult
        self.onError = onError

        // Request authorization
        SFSpeechRecognizer.requestAuthorization { [weak self] status in
            guard let self = self else { return }
            DispatchQueue.main.async {
                switch status {
                case .authorized:
                    self.startRecognition()
                case .denied:
                    let msg = "Speech recognition permission denied"
                    msg.withCString { ptr in self.onError?(ptr) }
                case .restricted:
                    let msg = "Speech recognition restricted on this device"
                    msg.withCString { ptr in self.onError?(ptr) }
                case .notDetermined:
                    let msg = "Speech recognition not yet authorized"
                    msg.withCString { ptr in self.onError?(ptr) }
                @unknown default:
                    let msg = "Unknown authorization status"
                    msg.withCString { ptr in self.onError?(ptr) }
                }
            }
        }
    }

    private func startRecognition() {
        // Stop any existing task
        stopRecognition()

        guard let recognizer = recognizer, recognizer.isAvailable else {
            let msg = "Speech recognizer not available"
            msg.withCString { ptr in onError?(ptr) }
            return
        }

        // Request microphone permission
        AVCaptureDevice.requestAccess(for: .audio) { [weak self] granted in
            guard let self = self else { return }
            DispatchQueue.main.async {
                if granted {
                    self.beginRecognition(recognizer: recognizer)
                } else {
                    let msg = "Microphone permission denied"
                    msg.withCString { ptr in self.onError?(ptr) }
                }
            }
        }
    }

    private func beginRecognition(recognizer: SFSpeechRecognizer) {
        let audioEngine = AVAudioEngine()
        let inputNode = audioEngine.inputNode

        let recognitionRequest = SFSpeechAudioBufferRecognitionRequest()
        recognitionRequest.shouldReportPartialResults = true
        // Keep recognition alive for continuous listening
        if #available(macOS 13.0, *) {
            recognitionRequest.addsPunctuation = true
        }
        self.recognitionRequest = recognitionRequest

        recognitionTask = recognizer.recognitionTask(with: recognitionRequest) { [weak self] result, error in
            guard let self = self else { return }

            if let error = error {
                // Cancellation is expected when we stop, don't report it as error
                let nsError = error as NSError
                if nsError.domain == NSURLErrorDomain && nsError.code == -999 {
                    // Recognition cancelled, this is normal
                    return
                }
                let msg = "Recognition error: \(error.localizedDescription)"
                msg.withCString { ptr in self.onError?(ptr) }
                return
            }

            if let result = result {
                let transcription = result.bestTranscription.formattedString
                let isFinal = result.isFinal

                transcription.withCString { ptr in
                    self.onResult?(ptr, isFinal)
                }

                if isFinal {
                    self.cancelSilenceTimer()
                    self.lastRecognizedText = ""
                    self.restartListening()
                } else if !transcription.isEmpty && transcription != self.lastRecognizedText {
                    self.lastRecognizedText = transcription
                    self.scheduleSilenceTimer()
                }
            }
        }

        // Configure audio input
        let recordingFormat = inputNode.outputFormat(forBus: 0)
        inputNode.installTap(onBus: 0, bufferSize: 1024, format: recordingFormat) { buffer, _ in
            recognitionRequest.append(buffer)
        }

        do {
            try audioEngine.start()
            self.audioEngine = audioEngine
            self.isListening = true
        } catch {
            let msg = "Audio engine start failed: \(error.localizedDescription)"
            msg.withCString { ptr in onError?(ptr) }
            self.cleanup()
        }
    }

    private func scheduleSilenceTimer() {
        // Recognition callback runs on a background queue without its own run loop,
        // so Timer.scheduledTimer there would never fire. Hop to main, which has
        // the default run loop, before installing the timer.
        DispatchQueue.main.async { [weak self] in
            guard let self = self else { return }
            self.silenceTimer?.invalidate()
            let timer = Timer.scheduledTimer(withTimeInterval: self.silenceThreshold, repeats: false) { [weak self] _ in
                guard let self = self, self.isListening else { return }
                // endAudio() forces SFSpeechRecognizer to flush isFinal=true on
                // the next callback, which cascades into restartListening().
                self.recognitionRequest?.endAudio()
            }
            RunLoop.main.add(timer, forMode: .common)
            self.silenceTimer = timer
        }
    }

    private func cancelSilenceTimer() {
        DispatchQueue.main.async { [weak self] in
            self?.silenceTimer?.invalidate()
            self?.silenceTimer = nil
        }
    }

    private func restartListening() {
        // Clean up current recognition but keep listening
        guard isListening else { return }

        cancelSilenceTimer()
        lastRecognizedText = ""

        if let audioEngine = audioEngine {
            audioEngine.inputNode.removeTap(onBus: 0)
        }
        audioEngine?.stop()
        audioEngine = nil

        recognitionTask?.cancel()
        recognitionTask = nil
        recognitionRequest = nil

        // Restart with a new request
        if let recognizer = recognizer, recognizer.isAvailable {
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { [weak self] in
                guard let self = self, self.isListening else { return }
                self.beginRecognition(recognizer: recognizer)
            }
        }
    }

    public func stopListening() {
        isListening = false
        stopRecognition()
    }

    private func stopRecognition() {
        cancelSilenceTimer()
        lastRecognizedText = ""

        if let audioEngine = audioEngine {
            audioEngine.inputNode.removeTap(onBus: 0)
            audioEngine.stop()
        }
        audioEngine = nil

        recognitionRequest?.endAudio()
        recognitionTask?.cancel()
        recognitionTask = nil
        recognitionRequest = nil
    }

    private func cleanup() {
        isListening = false
        cancelSilenceTimer()
        lastRecognizedText = ""
        audioEngine = nil
        recognitionRequest = nil
        recognitionTask = nil
    }

    public func getIsListening() -> Bool {
        return isListening
    }
}

// MARK: - C-callable interface

private var currentRecognizer: NativeSpeechRecognizer?
private var retainedOnResult: ResultCallback?
private var retainedOnError: ErrorCallback?

@_cdecl("speech_recognizer_create")
public func speechRecognizerCreate() -> UnsafeMutableRawPointer {
    let recognizer = NativeSpeechRecognizer()
    let pointer = Unmanaged.passRetained(recognizer).toOpaque()
    return pointer
}

@_cdecl("speech_recognizer_start")
public func speechRecognizerStart(
    ptr: UnsafeMutableRawPointer,
    onResult: @escaping ResultCallback,
    onError: @escaping ErrorCallback
) {
    let recognizer = Unmanaged<NativeSpeechRecognizer>.fromOpaque(ptr).takeUnretainedValue()
    // Retain callbacks since Swift needs them alive
    retainedOnResult = onResult
    retainedOnError = onError
    recognizer.startListening(onResult: onResult, onError: onError)
}

@_cdecl("speech_recognizer_stop")
public func speechRecognizerStop(ptr: UnsafeMutableRawPointer) {
    let recognizer = Unmanaged<NativeSpeechRecognizer>.fromOpaque(ptr).takeUnretainedValue()
    recognizer.stopListening()
    retainedOnResult = nil
    retainedOnError = nil
}

@_cdecl("speech_recognizer_destroy")
public func speechRecognizerDestroy(ptr: UnsafeMutableRawPointer) {
    let recognizer = Unmanaged<NativeSpeechRecognizer>.fromOpaque(ptr).takeRetainedValue()
    recognizer.stopListening()
}