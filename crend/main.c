#include <string.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <vulkan.h>

const char *VALIDATION_LAYER_NAME = "VK_LAYER_KHRONOS_validation";

#define SUCCESS 0
#define ERROR_INSTANCE_CREATION 1
#define ERROR_VALIDATION_LAYERS 2
#define ERROR_VULKAN_UNSUPPORTED 3
#define ERROR_NO_COMPATIBLE_QUEUE 4
#define ERROR_CANNOT_CREATE_LOGICAL_DEVICE 5

VkInstance instance;
VkPhysicalDevice physicalDevice = VK_NULL_HANDLE;
uint32_t queueFamilyIndex;
VkDevice device;
VkQueue primaryQueue;

bool checkValidationLayerSupport() {
    uint32_t layerCount;
    vkEnumerateInstanceLayerProperties(&layerCount, NULL);

    VkLayerProperties *availableLayers = malloc(sizeof(VkLayerProperties) * layerCount);
    vkEnumerateInstanceLayerProperties(&layerCount, availableLayers);
    for (uint32_t i = 0; i < layerCount; i++) {
        if (strcmp(VALIDATION_LAYER_NAME, availableLayers[i].layerName) == 0) {
            free(availableLayers);
            return true;
        }
    }

    free(availableLayers);
    return false;
}

uint32_t initInstance(
    const char **requiredExtensions, 
    uint32_t requiredExtensionsLen, 
    bool useValidationLayers
) {
    VkApplicationInfo appInfo = {};
    appInfo.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
    appInfo.pApplicationName = "Test";
    appInfo.applicationVersion = VK_MAKE_VERSION(1, 0, 0);
    appInfo.pEngineName = "No Engine";
    appInfo.engineVersion = VK_MAKE_VERSION(1, 0, 0);
    appInfo.apiVersion = VK_API_VERSION_1_0;

    VkInstanceCreateInfo createInfo = {};
    createInfo.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    createInfo.pApplicationInfo = &appInfo;
    createInfo.enabledExtensionCount = requiredExtensionsLen;
    createInfo.ppEnabledExtensionNames = requiredExtensions;

    const char **validationLayers = &VALIDATION_LAYER_NAME;
    if (useValidationLayers) {
        if (!checkValidationLayerSupport()) {
            return ERROR_VALIDATION_LAYERS;
        }
        createInfo.enabledLayerCount = 1;
        createInfo.ppEnabledLayerNames = validationLayers;
        printf("Enabled validation layers.\n");
    } else {
        createInfo.enabledLayerCount = 0;
    }

    VkResult result = vkCreateInstance(&createInfo, NULL, &instance);
    if (result != VK_SUCCESS) {
        return ERROR_INSTANCE_CREATION;
    }

    return SUCCESS;
}

uint32_t initPhysicalDevice() {
    uint32_t deviceCount = 0;
    vkEnumerateInstanceLayerProperties(&deviceCount, NULL);
    if (deviceCount == 0) {
        return ERROR_VULKAN_UNSUPPORTED;
    }

    VkPhysicalDevice *availableDevices = malloc(sizeof(VkPhysicalDevice) * deviceCount);
    vkEnumeratePhysicalDevices(instance, &deviceCount, availableDevices);
    for (uint32_t i = 0; i < deviceCount; i++) {
        physicalDevice = availableDevices[i];
        break;
    }
    free(availableDevices);

    uint32_t queueFamilyCount = 0;
    vkGetPhysicalDeviceQueueFamilyProperties(physicalDevice, &queueFamilyCount, NULL);
    VkQueueFamilyProperties *queueFamilies = malloc(sizeof(VkQueueFamilyProperties) * queueFamilyCount);
    vkGetPhysicalDeviceQueueFamilyProperties(physicalDevice, &queueFamilyCount, queueFamilies);
    VkQueueFlags requiredFlags = VK_QUEUE_GRAPHICS_BIT | VK_QUEUE_COMPUTE_BIT | VK_QUEUE_TRANSFER_BIT;
    for (uint32_t i = 0; i < queueFamilyCount; i++) {
        if ((queueFamilies[i].queueFlags & requiredFlags) == requiredFlags) {
            queueFamilyIndex = i;
            free(queueFamilies);
            return SUCCESS;
        }
    }
    free(queueFamilies);

    return ERROR_NO_COMPATIBLE_QUEUE;
}

uint32_t initLogicalDevice(bool useValidationLayers) {
    float queuePriority = 1.0f;

    VkDeviceQueueCreateInfo queueCreateInfo = {};
    queueCreateInfo.sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO;
    queueCreateInfo.queueFamilyIndex = queueFamilyIndex;
    queueCreateInfo.queueCount = 1;
    queueCreateInfo.pQueuePriorities = &queuePriority;

    VkPhysicalDeviceFeatures enabledFeatures = {};

    VkDeviceCreateInfo deviceCreateInfo = {};
    deviceCreateInfo.pQueueCreateInfos = &queueCreateInfo;
    deviceCreateInfo.queueCreateInfoCount = 1;
    deviceCreateInfo.pEnabledFeatures = &enabledFeatures;
    deviceCreateInfo.enabledExtensionCount = 0;

    // Enable device-specific validation layers for backwards compatibility with old Vulkan versions.
    const char **validationLayers = &VALIDATION_LAYER_NAME;
    if (useValidationLayers) {
        if (!checkValidationLayerSupport()) {
            return ERROR_VALIDATION_LAYERS;
        }
        deviceCreateInfo.enabledLayerCount = 1;
        deviceCreateInfo.ppEnabledLayerNames = validationLayers;
    } else {
        deviceCreateInfo.enabledLayerCount = 0;
    }

    if (vkCreateDevice(physicalDevice, &deviceCreateInfo, NULL, &device) != VK_SUCCESS) {
        return ERROR_CANNOT_CREATE_LOGICAL_DEVICE;
    }

    vkGetDeviceQueue(device, queueFamilyIndex, 0, &primaryQueue);

    return SUCCESS;
}

uint32_t crendInit(
    const char **requiredExtensions, 
    uint32_t requiredExtensionsLen, 
    bool useValidationLayers
) {
    uint32_t result = initInstance(requiredExtensions, requiredExtensionsLen, useValidationLayers);
    if (result != SUCCESS) return result;

    result = initPhysicalDevice();
    if (result != SUCCESS) return result;

    result = initLogicalDevice(useValidationLayers);
    if (result != SUCCESS) return result;

    return SUCCESS;
}

void crendDestroy() {
    vkDestroyDevice(device, NULL);
    vkDestroyInstance(instance, NULL);
}