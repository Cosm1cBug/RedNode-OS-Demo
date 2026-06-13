import 'package:firebase_core/firebase_core.dart';
import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'dart:convert';

// Background handler – must be top-level
@pragma('vm:entry-point')
Future<void> firebaseMessagingBackgroundHandler(RemoteMessage message) async {
  await Firebase.initializeApp();
  // RedNode approval push received in background
  // OS will show notification via FCM – nothing else needed
}

class FirebaseMessagingService {
  static final _messaging = FirebaseMessaging.instance;
  static final _localNotifications = FlutterLocalNotificationsPlugin();

  static Future<void> init() async {
    // Initialize Firebase – requires google-services.json in android/app/
    try {
      await Firebase.initializeApp();
    } catch (e) {
      // Firebase not configured – running in dev mode
      // To enable: flutterfire configure
      // See: interfaces/mobile/FIREBASE_SETUP.md
      return;
    }

    // Local notifications – for foreground approval pushes
    const androidInit = AndroidInitializationSettings('@mipmap/ic_launcher');
    const iosInit = DarwinInitializationSettings();
    await _localNotifications.initialize(
      const InitializationSettings(android: androidInit, iOS: iosInit),
      onDidReceiveNotificationResponse: (resp) {
        // User tapped approval notification
        // Navigate to approvals page – handled via navigatorKey in production
      },
    );

    // Request permission – iOS / Android 13+
    await _messaging.requestPermission(alert: true, badge: true, sound: true, provisional: false);

    // FCM token – send to RedNode CNS so it can push approvals to this device
    final token = await _messaging.getToken();
    if (token != null) {
      // TODO: POST /api/mobile/register {fcm_token: token}
      // RedNode CNS will store it in memory_longterm and use Firebase Admin SDK to push
      print('[FCM] device token: $token');
    }

    // Foreground messages – show local notification
    FirebaseMessaging.onMessage.listen((RemoteMessage message) {
      final notification = message.notification;
      final data = message.data;
      if (notification != null) {
        _showLocalNotification(
          notification.title ?? 'RedNode',
          notification.body ?? '',
          jsonEncode(data),
        );
      }
      // If this is an approval request – data['type'] == 'approval'
      // Show high-priority notification with Approve/Deny actions
    });

    // Background / terminated tap handling
    FirebaseMessaging.onMessageOpenedApp.listen((message) {
      // Navigate to approvals page
    });

    // Background handler
    FirebaseMessaging.onBackgroundMessage(firebaseMessagingBackgroundHandler);

    print('[FCM] Firebase Messaging initialized');
  }

  static Future<void> _showLocalNotification(String title, String body, String payload) async {
    const androidDetails = AndroidNotificationDetails(
      'rednode_approvals',
      'RedNode Approvals',
      channelDescription: 'High-priority approval requests from your RedNode',
      importance: Importance.max,
      priority: Priority.high,
      category: AndroidNotificationCategory.call,
      visibility: NotificationVisibility.public,
      // Add action buttons – Approve / Deny – requires extra setup
    );
    const details = NotificationDetails(android: androidDetails, iOS: DarwinNotificationDetails(presentAlert: true, presentBadge: true, presentSound: true, interruptionLevel: InterruptionLevel.critical));
    await _localNotifications.show(
      DateTime.now().millisecondsSinceEpoch ~/ 1000,
      title,
      body,
      details,
      payload: payload,
    );
  }

  static Future<String?> getToken() async {
    try { return await _messaging.getToken(); } catch (_) { return null; }
  }
}
